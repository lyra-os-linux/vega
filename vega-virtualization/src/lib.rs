//! Local VM operations. Libvirt enforces authorization; this code never runs as root.
//! UI callers must run operations on a worker, never the GTK main thread.
use std::path::{Path, PathBuf};
use virt::{connect::Connect, domain::Domain};
mod create;
pub use create::CreateRequest;
mod media;
pub use media::{Firmware, MediaKind};
mod shortcut;
pub use shortcut::create_shortcut;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Personal,
    System,
}
impl Connection {
    pub fn uri(self) -> &'static str {
        match self {
            Self::Personal => "qemu:///session",
            Self::System => "qemu:///system",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Paused,
    Off,
    Other,
}
impl State {
    fn from_raw(state: u32) -> Self {
        match state {
            1 | 2 => Self::Running,
            3 => Self::Paused,
            5 => Self::Off,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub uuid: String,
    pub name: String,
    pub state: State,
    pub cpus: u32,
    pub memory_kib: u64,
    pub persistent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start,
    Shutdown,
    Pause,
    Resume,
    ForceStop,
    Remove,
}
impl Action {
    pub fn allowed(self, state: State, persistent: bool) -> bool {
        match self {
            Self::Start => state == State::Off,
            Self::Shutdown | Self::Pause => state == State::Running,
            Self::Resume => state == State::Paused,
            Self::ForceStop => matches!(state, State::Running | State::Paused),
            Self::Remove => state == State::Off && persistent,
        }
    }
}

/// Invalid relative XDG paths are ignored, as required by the XDG specification.
pub fn data_directory(xdg: Option<&Path>, home: &Path) -> Result<PathBuf> {
    if let Some(base) = xdg.filter(|p| p.is_absolute()) {
        return Ok(base.join("lyra-vms"));
    }
    if !home.is_absolute() {
        return Err("HOME must be an absolute path".into());
    }
    Ok(home.join(".local/share/lyra-vms"))
}

pub struct Backend {
    conn: Connect,
}
impl Backend {
    /// Update a stopped persistent definition in one libvirt operation. Reject
    /// advanced CPU/memory layouts rather than silently discarding their rules.
    pub fn configure(&self, uuid: &str, cpus: u32, memory_mib: u64) -> Result<()> {
        uuid::Uuid::parse_str(uuid).map_err(|e| e.to_string())?;
        let domain = Domain::lookup_by_uuid_string(&self.conn, uuid).map_err(error)?;
        if domain.is_active().map_err(error)?
            || !domain.is_persistent().map_err(error)?
            || domain.has_managed_save(0).map_err(error)?
        {
            return Err("Shut down the persistent machine and discard any saved state before editing resources.".into());
        }
        let node = self.conn.get_node_info().map_err(error)?;
        if cpus == 0 || cpus > node.cpus || memory_mib < 512 || memory_mib > node.memory / 1024 {
            return Err("The requested CPU or memory exceeds this host's capacity.".into());
        }
        let xml = domain
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .map_err(error)?;
        let updated = resource_xml(&xml, cpus, memory_mib)?;
        Domain::define_xml(&self.conn, &updated).map_err(error)?;
        Ok(())
    }
    pub fn open(connection: Connection, read_only: bool) -> Result<Self> {
        let conn = if read_only {
            Connect::open_read_only(Some(connection.uri()))
        } else {
            Connect::open(Some(connection.uri()))
        }
        .map_err(error)?;
        Ok(Self { conn })
    }

    pub fn list(&self) -> Result<Vec<Machine>> {
        let mut machines = self
            .conn
            .list_all_domains(0)
            .map_err(error)?
            .into_iter()
            .map(|domain| {
                let info = domain.get_info().map_err(error)?;
                Ok(Machine {
                    uuid: domain.get_uuid_string().map_err(error)?,
                    name: domain.get_name().map_err(error)?,
                    state: State::from_raw(domain.get_state().map_err(error)?.0),
                    cpus: info.nr_virt_cpu,
                    memory_kib: info.max_mem,
                    persistent: domain.is_persistent().map_err(error)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        machines.sort_by_key(|m| m.name.to_lowercase());
        Ok(machines)
    }

    pub fn act(&self, uuid: &str, action: Action) -> Result<()> {
        uuid::Uuid::parse_str(uuid).map_err(|e| e.to_string())?;
        let domain = Domain::lookup_by_uuid_string(&self.conn, uuid).map_err(error)?;
        let state = State::from_raw(domain.get_state().map_err(error)?.0);
        if !action.allowed(state, domain.is_persistent().map_err(error)?) {
            return Err("The machine state changed. Refresh and try again.".into());
        }
        match action {
            Action::Start => domain.create().map(|_| ()).map_err(error),
            Action::Shutdown => domain.shutdown().map(|_| ()).map_err(error),
            Action::Pause => domain.suspend().map(|_| ()).map_err(error),
            Action::Resume => domain.resume().map(|_| ()).map_err(error),
            Action::ForceStop => domain.destroy().map_err(error),
            // Preserve NVRAM explicitly. Saved state/snapshots still require
            // special treatment; never delete disks or firmware here.
            Action::Remove => {
                let xml = domain
                    .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
                    .map_err(error)?;
                let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
                if doc.descendants().any(|n| n.has_tag_name("nvram")) {
                    domain
                        .undefine_flags(virt::sys::VIR_DOMAIN_UNDEFINE_KEEP_NVRAM)
                        .map_err(error)
                } else {
                    domain.undefine().map_err(error)
                }
            }
        }
    }
}
fn error(e: virt::error::Error) -> String {
    e.to_string()
}

fn resource_xml(xml: &str, cpus: u32, memory_mib: u64) -> Result<String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    if doc.descendants().any(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "topology"
                    | "numa"
                    | "numatune"
                    | "cputune"
                    | "vcpus"
                    | "memoryBacking"
                    | "maxMemory"
            )
    }) {
        return Err(
            "This machine has an advanced CPU or memory layout. Edit it in virt-manager.".into(),
        );
    }
    let mut patches = Vec::new();
    let mut memory_found = false;
    let mut cpu_found = false;
    for node in doc.root_element().children().filter(|n| n.is_element()) {
        let replacement = match node.tag_name().name() {
            "memory" => {
                memory_found = true;
                format!("<memory unit='MiB'>{memory_mib}</memory>")
            }
            "currentMemory" => format!("<currentMemory unit='MiB'>{memory_mib}</currentMemory>"),
            "vcpu" => {
                if node.attributes().any(|a| !matches!(a.name(), "placement"))
                    || node.attribute("placement").is_some_and(|s| s != "static")
                {
                    return Err(
                        "This machine has custom CPU placement. Edit it in virt-manager.".into(),
                    );
                }
                cpu_found = true;
                format!("<vcpu placement='static'>{cpus}</vcpu>")
            }
            _ => continue,
        };
        patches.push((node.range(), replacement));
    }
    if !memory_found || !cpu_found {
        return Err("The machine definition has no editable CPU/memory fields.".into());
    }
    let mut result = xml.to_string();
    for (range, replacement) in patches.into_iter().rev() {
        result.replace_range(range, &replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xdg_paths() {
        assert_eq!(
            data_directory(None, Path::new("/home/alice")).unwrap(),
            Path::new("/home/alice/.local/share/lyra-vms")
        );
        assert_eq!(
            data_directory(Some(Path::new("/data")), Path::new("/home/alice")).unwrap(),
            Path::new("/data/lyra-vms")
        );
        assert_eq!(
            data_directory(Some(Path::new("relative")), Path::new("/home/alice")).unwrap(),
            Path::new("/home/alice/.local/share/lyra-vms")
        );
        assert!(data_directory(None, Path::new("relative")).is_err());
    }
    #[test]
    fn safe_lifecycle_on_libvirt_test_driver() {
        let backend = Backend {
            conn: Connect::open(Some("test:///default")).unwrap(),
        };
        let machine = backend.list().unwrap().remove(0);
        assert_eq!(machine.state, State::Running);
        assert!(backend.act(&machine.uuid, Action::Remove).is_err());
        backend.act(&machine.uuid, Action::Pause).unwrap();
        assert_eq!(backend.list().unwrap()[0].state, State::Paused);
        assert!(backend.act(&machine.uuid, Action::Start).is_err());
        backend.act(&machine.uuid, Action::Resume).unwrap();
        backend.act(&machine.uuid, Action::Shutdown).unwrap();
        backend.configure(&machine.uuid, 1, 512).unwrap();
        let updated = backend.list().unwrap().remove(0);
        assert_eq!((updated.cpus, updated.memory_kib), (1, 512 * 1024));
        backend.act(&machine.uuid, Action::Start).unwrap();
        backend.act(&machine.uuid, Action::ForceStop).unwrap();
        backend.act(&machine.uuid, Action::Remove).unwrap();
        assert!(backend.list().unwrap().is_empty());
        assert!(backend.act(&machine.uuid, Action::Start).is_err());
    }
    #[test]
    fn resource_edit_preserves_devices_and_rejects_custom_layouts() {
        let xml = "<domain><memory unit='KiB'>2048</memory><currentMemory>2048</currentMemory><vcpu>2</vcpu><devices><disk><source file='/keep&amp;this'/></disk></devices></domain>";
        let updated = resource_xml(xml, 4, 4096).unwrap();
        assert!(
            updated.contains("<devices><disk><source file='/keep&amp;this'/></disk></devices>")
        );
        assert!(updated.contains("<currentMemory unit='MiB'>4096</currentMemory>"));
        assert!(resource_xml(&xml.replace("<vcpu>", "<vcpu cpuset='1'>"), 1, 512).is_err());
        assert!(
            resource_xml(
                &xml.replace(
                    "<devices>",
                    "<cpu><topology sockets='1' cores='2' threads='1'/></cpu><devices>"
                ),
                1,
                512
            )
            .is_err()
        );
    }
}
