use super::{Backend, Firmware, MediaKind, Result, data_directory, error};
use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use virt::{domain::Domain, storage_pool::StoragePool, storage_vol::StorageVol, stream::Stream};

#[derive(Clone, Debug)]
pub struct CreateRequest {
    pub name: String,
    pub source: PathBuf,
    pub kind: MediaKind,
    pub firmware: Firmware,
    pub cpus: u32,
    pub memory_mib: u64,
    pub disk_gib: u64,
}

impl CreateRequest {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty()
            || self.name.len() > 128
            || self.name.chars().any(char::is_control)
        {
            return Err("Enter a machine name (1–128 bytes, no control characters).".into());
        }
        if !(1..=64).contains(&self.cpus)
            || !(512..=262144).contains(&self.memory_mib)
            || !(4..=2048).contains(&self.disk_gib)
        {
            return Err("CPU, memory or disk size is outside the supported range.".into());
        }
        if !self.source.is_absolute() {
            return Err("Select an absolute media path.".into());
        }
        Ok(())
    }
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

impl Backend {
    /// Create an OFF personal machine. Sources are copied, never moved or removed.
    /// A cancellation after successful definition does not remove the new machine.
    pub fn create(&self, request: &CreateRequest, cancel: &AtomicBool) -> Result<String> {
        self.create_with_progress(request, cancel, |_, _| {})
    }

    pub fn create_with_progress(
        &self,
        request: &CreateRequest,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<String> {
        request.validate()?;
        if self.conn.get_uri().map_err(error)? != "qemu:///session" {
            return Err("Create personal machines in the personal connection.".into());
        }
        let node = self.conn.get_node_info().map_err(error)?;
        if u64::from(request.cpus) > u64::from(node.cpus) || request.memory_mib > node.memory / 1024
        {
            return Err("The requested CPU or memory exceeds this host's capacity.".into());
        }
        if self.list()?.iter().any(|m| m.name == request.name.trim()) {
            return Err("A machine with this name already exists.".into());
        }
        let mut source = File::open(&request.source).map_err(|e| e.to_string())?;
        let metadata = source.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err("The installation media must be a nonempty regular file.".into());
        }
        let mut prefix = vec![0; metadata.len().min(104) as usize];
        source.read_exact(&mut prefix).map_err(|e| e.to_string())?;
        request.kind.check_prefix(&prefix)?;
        source.rewind().map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Creation cancelled.".into());
        }
        let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
        let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
        let root = data_directory(xdg.as_deref(), &PathBuf::from(home))?;
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let path = root.to_str().ok_or("The storage path must be UTF-8")?;
        let pools = self.conn.list_all_storage_pools(0).map_err(error)?;
        let existing = pools
            .into_iter()
            .find(|p| p.get_name().ok().as_deref() == Some("lyra-vms"));
        let pool = if let Some(pool) = existing {
            let xml = pool.get_xml_desc(0).map_err(error)?;
            let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
            let target = doc
                .root_element()
                .children()
                .find(|n| n.has_tag_name("target"))
                .and_then(|n| n.children().find(|n| n.has_tag_name("path")))
                .and_then(|n| n.text());
            if doc.root_element().attribute("type") != Some("dir") || target != Some(path) {
                return Err(
                    "The existing lyra-vms storage pool uses another path. It was not changed."
                        .into(),
                );
            }
            pool
        } else {
            StoragePool::define_xml(
                &self.conn,
                &format!(
                    "<pool type='dir'><name>lyra-vms</name><target><path>{}</path></target></pool>",
                    escape(path)
                ),
                0,
            )
            .map_err(error)?
        };
        if !pool.is_active().map_err(error)? {
            pool.create(0).map_err(error)?;
        }
        let uuid = uuid::Uuid::new_v4().to_string();
        let mut created: Vec<StorageVol> = Vec::new();
        let result = (|| {
            if cancel.load(Ordering::Relaxed) {
                return Err("Creation cancelled.".into());
            }
            let (disk, media) = if request.kind == MediaKind::Iso {
                let disk_xml = format!(
                    "<volume><name>{uuid}.qcow2</name><capacity unit='GiB'>{}</capacity><allocation>0</allocation><target><format type='qcow2'/></target></volume>",
                    request.disk_gib
                );
                created.push(StorageVol::create_xml(&pool, &disk_xml, 0).map_err(error)?);
                let disk = created[0].get_path().map_err(error)?;
                let iso_xml = format!(
                    "<volume><name>{uuid}.iso</name><capacity unit='bytes'>{}</capacity><target><format type='raw'/></target></volume>",
                    metadata.len()
                );
                created.push(StorageVol::create_xml(&pool, &iso_xml, 0).map_err(error)?);
                (disk, created[1].get_path().map_err(error)?)
            } else {
                // Upload exact file bytes, without conversion or following backing files.
                let xml = format!(
                    "<volume><name>{uuid}.{}</name><capacity unit='bytes'>{}</capacity><allocation>0</allocation><target><format type='raw'/></target></volume>",
                    request.kind.disk_format(),
                    metadata.len()
                );
                created.push(StorageVol::create_xml(&pool, &xml, 0).map_err(error)?);
                (created[0].get_path().map_err(error)?, String::new())
            };
            let stream = Stream::new(&self.conn, 0).map_err(error)?;
            created
                .last()
                .expect("created upload volume")
                .upload(&stream, 0, metadata.len(), 0)
                .map_err(error)?;
            let upload: Result<()> = (|| {
                let mut buffer = vec![0u8; 1024 * 1024];
                let mut remaining = metadata.len();
                while remaining > 0 {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("Creation cancelled.".into());
                    }
                    let limit = buffer.len().min(remaining as usize);
                    let count = source
                        .read(&mut buffer[..limit])
                        .map_err(|e| e.to_string())?;
                    if count == 0 {
                        return Err("The installation media changed while copying.".into());
                    }
                    if remaining == metadata.len() {
                        request.kind.check_prefix(&buffer[..count])?;
                        if count < prefix.len() || buffer[..prefix.len()] != prefix {
                            return Err("The source media changed while copying.".into());
                        }
                    }
                    let mut sent = 0;
                    while sent < count {
                        let n = stream.send(&buffer[sent..count]).map_err(error)?;
                        if n == 0 {
                            return Err("Media transfer stopped unexpectedly.".into());
                        }
                        sent += n;
                    }
                    remaining -= count as u64;
                    progress(metadata.len() - remaining, metadata.len());
                }
                Ok(())
            })();
            if let Err(e) = upload {
                let _ = stream.abort();
                return Err(e);
            }
            stream.finish().map_err(error)?;
            let after = source.metadata().map_err(|e| e.to_string())?;
            if after.len() != metadata.len() || after.modified().ok() != metadata.modified().ok() {
                return Err("The source media changed while copying. Shut down its machine before importing.".into());
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("Creation cancelled.".into());
            }
            let xml = domain_xml(request, &uuid, &disk, &media);
            Domain::define_xml(&self.conn, &xml).map_err(error)?;
            Ok(uuid)
        })();
        match result {
            Ok(uuid) => Ok(uuid),
            Err(mut message) => {
                // Only handles returned by successful creation are eligible for rollback.
                for volume in created.into_iter().rev() {
                    if let Err(e) = volume.delete(0) {
                        message.push_str(&format!(
                            "\nCould not remove new volume {:?}: {e}",
                            volume.get_path()
                        ));
                    }
                }
                Err(message)
            }
        }
    }
}

fn domain_xml(r: &CreateRequest, uuid: &str, disk: &str, iso: &str) -> String {
    let firmware = match r.firmware {
        Firmware::Bios => "",
        Firmware::Uefi => " firmware='efi'",
    };
    let firmware_options = if r.firmware == Firmware::Uefi {
        "<firmware><feature enabled='no' name='secure-boot'/></firmware>"
    } else {
        ""
    };
    let cdrom = if r.kind == MediaKind::Iso {
        format!(
            "<disk type='file' device='cdrom'><driver name='qemu' type='raw'/><source file='{}'/><target dev='sda' bus='sata'/><readonly/></disk>",
            escape(iso)
        )
    } else {
        String::new()
    };
    let cdrom_boot = if r.kind == MediaKind::Iso {
        "<boot dev='cdrom'/>"
    } else {
        ""
    };
    let disk_format = r.kind.disk_format();
    format!(
        "<domain type='kvm'><name>{}</name><uuid>{uuid}</uuid>
      <memory unit='MiB'>{}</memory><vcpu>{}</vcpu>
      <os{firmware}><type arch='x86_64' machine='q35'>hvm</type>{firmware_options}{cdrom_boot}<boot dev='hd'/></os>
      <features><acpi/><apic/></features><cpu mode='host-model'/>
      <devices><disk type='file' device='disk'><driver name='qemu' type='{disk_format}'/>
        <source file='{}'/><target dev='vda' bus='virtio'/></disk>
        {cdrom}
        <interface type='user'><model type='virtio'/></interface>
        <controller type='usb' model='qemu-xhci'/><input type='tablet' bus='usb'/>
        <graphics type='vnc'><listen type='socket'/></graphics><video><model type='virtio'/></video>
      </devices></domain>",
        escape(r.name.trim()),
        r.memory_mib,
        r.cpus,
        escape(disk)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_limits_and_escapes_user_text() {
        let mut r = CreateRequest {
            name: "A<&'\"".into(),
            source: "/tmp/a.iso".into(),
            kind: MediaKind::Iso,
            firmware: Firmware::Bios,
            cpus: 2,
            memory_mib: 2048,
            disk_gib: 20,
        };
        r.validate().unwrap();
        let xml = domain_xml(
            &r,
            "a84fdf47-f7f3-4306-9358-d2d3f38af37c",
            "/tmp/a'b",
            "/tmp/c&d",
        );
        let doc = roxmltree::Document::parse(&xml).unwrap();
        assert_eq!(
            doc.descendants()
                .find(|n| n.has_tag_name("name"))
                .unwrap()
                .text(),
            Some(r.name.as_str())
        );
        assert!(!xml.contains("listen='0.0.0.0'"));
        r.disk_gib = u64::MAX;
        assert!(r.validate().is_err());
        r.disk_gib = 20;
        r.source = "relative.iso".into();
        assert!(r.validate().is_err());
    }
}
