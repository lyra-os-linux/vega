//! Offline edits; storage changes use libvirt and never enable shrinking.
use super::{Backend, Result, State, error};
use std::os::unix::fs::MetadataExt;
use virt::{domain::Domain, storage_vol::StorageVol};

#[derive(Clone, Debug)]
pub struct Disk {
    pub target: String,
    pub path: String,
    pub capacity: Option<u64>,
    pub unavailable: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct EditDetails {
    pub disks: Vec<Disk>,
    pub media: Vec<String>,
}
impl Backend {
    pub(super) fn editable(&self, uuid: &str) -> Result<Domain> {
        uuid::Uuid::parse_str(uuid).map_err(|e| e.to_string())?;
        let d = Domain::lookup_by_uuid_string(&self.conn, uuid).map_err(error)?;
        if State::from_raw(d.get_state().map_err(error)?.0) != State::Off
            || !d.is_persistent().map_err(error)?
            || d.has_managed_save(0).map_err(error)?
        {
            return Err("Shut down the persistent machine and discard any saved state before editing resources.".into());
        }
        Ok(d)
    }
    pub fn rename(&self, uuid: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err("Enter a machine name (1–128 bytes, no control characters).".into());
        }
        let d = self.editable(uuid)?;
        if d.get_name().map_err(error)? == name {
            return Ok(());
        }
        if self
            .list()?
            .iter()
            .any(|m| m.uuid != uuid && m.name == name)
        {
            return Err("A machine with this name already exists.".into());
        }
        d.rename(name, 0).map_err(error)?;
        Ok(())
    }
    pub fn edit_details(&self, uuid: &str) -> Result<EditDetails> {
        let d = self.editable(uuid)?;
        let xml = d
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .map_err(error)?;
        let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
        let mut details = EditDetails::default();
        for n in doc.descendants().filter(|n| n.has_tag_name("disk")) {
            let target = child(n, "target")
                .and_then(|n| n.attribute("dev"))
                .unwrap_or("");
            if target.is_empty() {
                continue;
            }
            if n.attribute("device") == Some("cdrom") {
                if child(n, "source").is_some() {
                    details.media.push(target.into());
                }
            } else if n.attribute("device") == Some("disk") {
                let path = child(n, "source")
                    .and_then(|n| n.attribute("file"))
                    .unwrap_or("");
                let result = self.growable_volume(&d, target);
                let (capacity, unavailable) = match result {
                    Ok(v) => (Some(v.get_info().map_err(error)?.capacity), None),
                    Err(e) => (None, Some(e)),
                };
                details.disks.push(Disk {
                    target: target.into(),
                    path: path.into(),
                    capacity,
                    unavailable,
                });
            }
        }
        Ok(details)
    }
    fn growable_volume(&self, d: &Domain, target: &str) -> Result<StorageVol> {
        let unsupported = || {
            "Only standalone, unshared RAW/QCOW2 file volumes without snapshots can be enlarged."
                .to_string()
        };
        if !d.list_all_snapshots(0).map_err(error)?.is_empty() {
            return Err(unsupported());
        }
        let xml = d
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .map_err(error)?;
        let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
        let disk = find_disk(&doc, target, "disk")?;
        let format = child(disk, "driver")
            .and_then(|n| n.attribute("type"))
            .ok_or_else(unsupported)?;
        if disk.attribute("type") != Some("file")
            || !matches!(format, "raw" | "qcow2")
            || disk.descendants().any(|n| {
                n.has_tag_name("shareable")
                    || n.has_tag_name("readonly")
                    || n.has_tag_name("encryption")
                    || (n.has_tag_name("backingStore") && n.children().any(|c| c.is_element()))
            })
        {
            return Err(unsupported());
        }
        let path = child(disk, "source")
            .and_then(|n| n.attribute("file"))
            .ok_or_else(unsupported)?;
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err(unsupported());
        }
        let uuid = d.get_uuid_string().map_err(error)?;
        if self.referenced_elsewhere(&uuid, path)? {
            return Err(unsupported());
        }
        // Include inactive definitions of other machines: sharing is unsafe even
        // when the other machine happens to be stopped. Compare inode aliases.
        for other in self.conn.list_all_domains(0).map_err(error)? {
            let same = other.get_uuid_string().map_err(error)? == uuid;
            let xml = other
                .get_xml_desc(if other.is_persistent().map_err(error)? {
                    virt::sys::VIR_DOMAIN_XML_INACTIVE
                } else {
                    0
                })
                .map_err(error)?;
            let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
            for n in doc.descendants().filter(|n| n.has_tag_name("disk")) {
                if same && child(n, "target").and_then(|n| n.attribute("dev")) == Some(target) {
                    continue;
                }
                for source in n.descendants().filter(|n| n.has_tag_name("source")) {
                    if let Some(p) = source.attribute("file") {
                        let m = std::fs::metadata(p).map_err(|e| e.to_string())?;
                        if (m.dev(), m.ino()) == (metadata.dev(), metadata.ino()) {
                            return Err(unsupported());
                        }
                    }
                }
            }
        }
        let volume = StorageVol::lookup_by_path(&self.conn, path).map_err(error)?;
        virt::storage_pool::StoragePool::lookup_by_volume(&volume)
            .map_err(error)?
            .refresh(0)
            .map_err(error)?;
        let volume = StorageVol::lookup_by_path(&self.conn, path).map_err(error)?;
        let xml = volume.get_xml_desc(0).map_err(error)?;
        let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
        if doc
            .descendants()
            .any(|n| n.has_tag_name("backingStore") || n.has_tag_name("encryption"))
            || !doc
                .descendants()
                .any(|n| n.has_tag_name("format") && n.attribute("type") == Some(format))
        {
            return Err(unsupported());
        }
        // Reject QCOW2 internal snapshots/external data not represented by domain XML.
        if format == "qcow2" {
            use std::io::Read;
            let mut header = [0; 104];
            std::fs::File::open(path)
                .and_then(|mut f| f.read_exact(&mut header))
                .map_err(|e| e.to_string())?;
            super::MediaKind::Qcow2.check_prefix(&header)?;
        }
        Ok(volume)
    }
    pub fn grow_disk(&self, uuid: &str, target: &str, bytes: u64) -> Result<()> {
        let d = self.editable(uuid)?;
        let volume = self.growable_volume(&d, target)?;
        validate_growth(volume.get_info().map_err(error)?.capacity, bytes)?;
        self.editable(uuid)?; // Recheck immediately before the storage operation.
        volume.resize(bytes, 0).map_err(error)?; // Never VIR_STORAGE_VOL_RESIZE_SHRINK.
        Ok(())
    }
    pub fn eject_iso(&self, uuid: &str, target: &str) -> Result<()> {
        let d = self.editable(uuid)?;
        let xml = d
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .map_err(error)?;
        let updated = eject_xml(&xml, target)?;
        Domain::define_xml(&self.conn, &updated).map_err(error)?;
        Ok(())
    }
}
fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|n| n.has_tag_name(name))
}
fn find_disk<'a, 'i>(
    doc: &'a roxmltree::Document<'i>,
    target: &str,
    device: &str,
) -> Result<roxmltree::Node<'a, 'i>> {
    doc.descendants()
        .find(|n| {
            n.has_tag_name("disk")
                && n.attribute("device") == Some(device)
                && child(*n, "target").and_then(|n| n.attribute("dev")) == Some(target)
        })
        .ok_or_else(|| "The selected disk or optical drive no longer exists.".into())
}
fn validate_growth(current: u64, requested: u64) -> Result<()> {
    if requested <= current || requested > (2048_u64 << 30) {
        return Err(
            "Choose a larger disk capacity, up to 2048 GiB. Shrinking is not allowed.".into(),
        );
    }
    Ok(())
}
fn eject_xml(xml: &str, target: &str) -> Result<String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let disk = find_disk(&doc, target, "cdrom")?;
    let mut result = xml.to_string();
    if let Some(source) = child(disk, "source") {
        result.replace_range(source.range(), "");
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn growth_never_shrinks_or_overflows() {
        assert!(validate_growth(4 << 30, 5 << 30).is_ok());
        for size in [0, 3 << 30, 4 << 30, u64::MAX] {
            assert!(validate_growth(4 << 30, size).is_err());
        }
    }
    #[test]
    fn eject_only_removes_selected_cdrom_source() {
        let xml = "<domain><devices><disk device='disk'><source file='/keep'/><target dev='vda'/></disk><disk device='cdrom'><source file='/iso'/><target dev='sda'/><readonly/></disk></devices></domain>";
        let result = eject_xml(xml, "sda").unwrap();
        assert_eq!(result, xml.replace("<source file='/iso'/>", ""));
        assert!(eject_xml(xml, "vda").is_err());
        assert_eq!(eject_xml(&result, "sda").unwrap(), result);
    }
}
