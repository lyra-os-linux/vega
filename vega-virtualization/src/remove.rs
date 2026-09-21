//! Explicit deletion plans. Never recursively remove directories or follow links.
use super::{Backend, Result, data_directory, error};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileIdentity {
    path: String,
    dev: u64,
    ino: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovalPlan {
    pub files: Vec<String>,
    volumes: Vec<FileIdentity>,
    nvram: Option<FileIdentity>,
    shortcut: Option<(FileIdentity, String)>,
}
fn identity(path: &str) -> Result<FileIdentity> {
    let m = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() || m.nlink() != 1 {
        return Err("Linked or non-regular files will not be deleted.".into());
    }
    Ok(FileIdentity {
        path: path.into(),
        dev: m.dev(),
        ino: m.ino(),
    })
}
impl Backend {
    pub(super) fn referenced_elsewhere(&self, uuid: &str, path: &str) -> Result<bool> {
        let target = std::fs::metadata(path).map_err(|e| e.to_string())?;
        for d in self.conn.list_all_domains(0).map_err(error)? {
            if d.get_uuid_string().map_err(error)? == uuid {
                continue;
            }
            let mut documents = vec![d.get_xml_desc(0).map_err(error)?];
            if d.is_persistent().map_err(error)? {
                documents.push(
                    d.get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
                        .map_err(error)?,
                );
            }
            for snapshot in d.list_all_snapshots(0).map_err(error)? {
                documents.push(snapshot.get_xml_desc(0).map_err(error)?);
            }
            for xml in documents {
                let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
                for n in doc.descendants().filter(|n| n.is_element()) {
                    let p = n.attribute("file").or_else(|| {
                        if n.has_tag_name("nvram") {
                            n.text()
                        } else {
                            None
                        }
                    });
                    if let Some(p) = p {
                        if p == path {
                            return Ok(true);
                        }
                        match std::fs::metadata(p) {
                            Ok(m) if (m.dev(), m.ino()) == (target.dev(), target.ino()) => {
                                return Ok(true);
                            }
                            Ok(_) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
            }
        }
        Ok(false)
    }
    pub fn removal_plan(&self, uuid: &str) -> Result<RemovalPlan> {
        let d = self.editable(uuid)?;
        if !d.list_all_snapshots(0).map_err(error)?.is_empty() {
            return Err("Remove snapshots before deleting the machine's local files.".into());
        }
        let mut plan = RemovalPlan {
            files: vec![],
            volumes: vec![],
            nvram: None,
            shortcut: None,
        };
        // Only the personal pool created by Vega establishes ownership of detached
        // ISO copies too. External disks are never inferred from a path prefix alone.
        if self.conn.get_uri().map_err(error)? == "qemu:///session" {
            let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
            let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
            let root = data_directory(xdg.as_deref(), Path::new(&home))?;
            if let Some(pool) = self
                .conn
                .list_all_storage_pools(0)
                .map_err(error)?
                .into_iter()
                .find(|p| p.get_name().ok().as_deref() == Some("lyra-vms"))
            {
                if !pool.is_active().map_err(error)? {
                    return Err("Activate the local storage pool before deleting its files.".into());
                }
                for volume in pool.list_all_volumes(0).map_err(error)? {
                    let name = volume.get_name().map_err(error)?;
                    if !["qcow2", "raw", "iso"]
                        .iter()
                        .any(|ext| name == format!("{uuid}.{ext}"))
                    {
                        continue;
                    }
                    let path = volume.get_path().map_err(error)?;
                    if Path::new(&path)
                        .parent()
                        .and_then(|p| p.canonicalize().ok())
                        != root.canonicalize().ok()
                    {
                        continue;
                    }
                    if self.referenced_elsewhere(uuid, &path)? {
                        continue;
                    }
                    plan.volumes.push(identity(&path)?);
                }
            }
            let shortcut = root
                .parent()
                .ok_or("Invalid data directory")?
                .join("applications")
                .join(format!("org.lyraos.VMs.personal.{uuid}.desktop"));
            if shortcut.exists() {
                let path = shortcut.to_str().ok_or("The storage path must be UTF-8")?;
                let content = std::fs::read_to_string(&shortcut).map_err(|e| e.to_string())?;
                let expected =
                    format!("Exec=/usr/bin/lyra-vms --connect qemu:///session --uuid {uuid}");
                if content
                    .lines()
                    .filter(|s| s.starts_with("Exec="))
                    .eq(std::iter::once(expected.as_str()))
                {
                    plan.shortcut = Some((identity(path)?, content));
                }
            }
        }
        let xml = d
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .map_err(error)?;
        let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
        if let Some(path) = doc
            .descendants()
            .find(|n| n.has_tag_name("nvram"))
            .and_then(|n| n.text())
        {
            match std::fs::symlink_metadata(path) {
                Ok(_) => {
                    if !self.referenced_elsewhere(uuid, path)? {
                        plan.nvram = Some(identity(path)?);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        plan.volumes.sort_by(|a, b| a.path.cmp(&b.path));
        plan.files = plan.volumes.iter().map(|f| f.path.clone()).collect();
        if let Some(f) = &plan.nvram {
            plan.files.push(f.path.clone());
        }
        if let Some((f, _)) = &plan.shortcut {
            plan.files.push(f.path.clone());
        }
        Ok(plan)
    }
    pub fn remove_with_files(&self, uuid: &str, approved: &RemovalPlan) -> Result<()> {
        // The exact plan shown in the confirmation must still be current.
        if &self.removal_plan(uuid)? != approved {
            return Err("The local file list changed. Review the removal again.".into());
        }
        let d = self.editable(uuid)?;
        let flags = if approved.nvram.is_some() {
            virt::sys::VIR_DOMAIN_UNDEFINE_NVRAM
        } else {
            virt::sys::VIR_DOMAIN_UNDEFINE_KEEP_NVRAM
        };
        d.undefine_flags(flags).map_err(error)?;
        let mut failures = Vec::new();
        for f in &approved.volumes {
            let result = (|| {
                if identity(&f.path)? != *f || self.referenced_elsewhere(uuid, &f.path)? {
                    return Err("File ownership changed; preserved.".to_string());
                }
                virt::storage_vol::StorageVol::lookup_by_path(&self.conn, &f.path)
                    .map_err(error)?
                    .delete(0)
                    .map_err(error)?;
                Ok(())
            })();
            if let Err(e) = result {
                failures.push(format!("{}: {e}", f.path));
            }
        }
        if let Some((f, content)) = &approved.shortcut {
            let result = (|| {
                if identity(&f.path)? != *f
                    || std::fs::read_to_string(&f.path).map_err(|e| e.to_string())? != *content
                {
                    return Err("File ownership changed; preserved.".to_string());
                }
                std::fs::remove_file(&f.path).map_err(|e| e.to_string())
            })();
            if let Err(e) = result {
                failures.push(format!("{}: {e}", f.path));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "The machine was removed, but some local files were preserved.\n{}",
                failures.join("\n")
            ))
        }
    }
}
