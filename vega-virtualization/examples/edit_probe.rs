//! Destructive tests run only in the marked disposable guest, never on the host.
use std::{path::Path, process::Command, sync::atomic::AtomicBool};
use vega_virtualization::{Action, Backend, Connection, CreateRequest, Firmware, MediaKind};
fn main() -> Result<(), String> {
    assert!(
        std::fs::read_to_string("/proc/cmdline")
            .unwrap()
            .split_whitespace()
            .any(|s| s == "lyra.virtualization-test=1")
    );
    let b = Backend::open(Connection::Personal, false)?;
    assert!(b.list()?.is_empty(), "fixture must start without machines");
    let request = CreateRequest {
        name: "Editing fixture".into(),
        source: "/tmp/lyra-vms-fixture.iso".into(),
        kind: MediaKind::Iso,
        firmware: Firmware::Uefi,
        cpus: 1,
        memory_mib: 512,
        disk_gib: 4,
    };
    let original = std::fs::read(&request.source).unwrap();
    let id = b.create(&request, &AtomicBool::new(false))?;
    let details = b.edit_details(&id)?;
    println!("DETAILS {details:?}");
    let disk = &details.disks[0];
    assert_eq!(disk.capacity, Some(4 << 30));
    let io = |command: &str| {
        assert!(
            Command::new("qemu-io")
                .args(["-f", "qcow2", "-c", command, &disk.path])
                .status()
                .unwrap()
                .success()
        );
    };
    io("write -P 0x5a 1048576 4096");
    b.rename(&id, "Edited & preserved")?;
    b.configure(&id, 2, 768)?;
    let m = b.list()?.remove(0);
    assert_eq!(m.uuid, id);
    assert_eq!(m.name, "Edited & preserved");
    assert_eq!((m.cpus, m.memory_kib), (2, 768 * 1024));
    assert!(b.rename(&id, "").is_err());
    assert!(b.grow_disk(&id, &disk.target, 3 << 30).is_err());
    assert!(b.grow_disk(&id, &disk.target, 4 << 30).is_err());
    b.grow_disk(&id, &disk.target, 5 << 30)?;
    assert_eq!(b.edit_details(&id)?.disks[0].capacity, Some(5 << 30));
    io("read -P 0x5a 1048576 4096");
    assert!(b.eject_iso(&id, &disk.target).is_err());
    b.eject_iso(&id, &details.media[0])?;
    assert!(b.edit_details(&id)?.media.is_empty());
    assert_eq!(std::fs::read(&request.source).unwrap(), original);
    // Shared disks cannot grow or enter another machine's deletion plan.
    let conn = virt::connect::Connect::open(Some(Connection::Personal.uri())).unwrap();
    let domain = virt::domain::Domain::lookup_by_uuid_string(&conn, &id).unwrap();
    let xml = domain
        .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
        .unwrap();
    let peer = uuid::Uuid::new_v4().to_string();
    // The peer only needs a disk definition, not cloned firmware state.
    let peer_xml = format!(
        "<domain type='kvm'><name>Shared fixture</name><uuid>{peer}</uuid><memory unit='MiB'>512</memory><vcpu>1</vcpu><os><type arch='x86_64'>hvm</type></os><devices><disk type='file' device='disk'><driver name='qemu' type='qcow2'/><source file='{}'/><target dev='vda' bus='virtio'/></disk></devices></domain>",
        disk.path
    );
    virt::domain::Domain::define_xml(&conn, &peer_xml).unwrap();
    assert!(b.grow_disk(&id, &disk.target, 6 << 30).is_err());
    assert!(!b.removal_plan(&id)?.files.contains(&disk.path));
    b.act(&peer, Action::Remove)?;
    b.act(&id, Action::Start)?;
    assert!(b.rename(&id, "Running rename").is_err());
    assert!(b.grow_disk(&id, &disk.target, 6 << 30).is_err());
    assert!(b.eject_iso(&id, &details.media[0]).is_err());
    assert!(b.removal_plan(&id).is_err());
    b.act(&id, Action::ForceStop)?;
    let plan = b.removal_plan(&id)?;
    assert!(plan.files.contains(&disk.path));
    assert!(plan.files.iter().any(|p| p.ends_with(".iso")));
    assert!(plan.files.iter().any(|p| p.contains("VARS")));
    // Default removal leaves every enumerated file behind.
    b.act(&id, Action::Remove)?;
    for f in &plan.files {
        assert!(Path::new(f).exists(), "preserve {f}");
    }
    virt::domain::Domain::define_xml(&conn, &xml).unwrap();
    // A newly added owned shortcut invalidates a previously confirmed list.
    vega_virtualization::create_shortcut(
        Connection::Personal,
        &id,
        "fixture",
        None,
        Path::new(&std::env::var("HOME").unwrap()),
    )?;
    assert!(b.remove_with_files(&id, &plan).is_err());
    assert_eq!(b.list()?.len(), 1);
    let plan = b.removal_plan(&id)?;
    println!("DELETE PLAN {:?}", plan.files);
    b.remove_with_files(&id, &plan)?;
    assert!(b.list()?.is_empty());
    for f in &plan.files {
        assert!(!Path::new(f).exists(), "delete {f}");
    }
    assert_eq!(std::fs::read(&request.source).unwrap(), original);
    println!(
        "PASS rename, CPU/RAM, grow-only and payload preservation, ISO ejection, shared/running guards, preserve default, stale-plan rejection, explicit local cleanup including detached ISO, NVRAM and shortcut"
    );
    for kind in [MediaKind::Raw, MediaKind::Qcow2] {
        let source = format!("/tmp/lyra-vms-fixture.{}", kind.disk_format());
        let original = std::fs::read(&source).unwrap();
        let request = CreateRequest {
            name: format!("Imported {kind:?}"),
            source: source.into(),
            kind,
            firmware: Firmware::Bios,
            ..request.clone()
        };
        let id = b.create(&request, &AtomicBool::new(false))?;
        let details = b.edit_details(&id)?;
        let disk = &details.disks[0];
        assert_eq!(disk.capacity, Some(4 << 20), "{details:?}");
        b.grow_disk(&id, &disk.target, 1 << 30)?;
        assert_eq!(b.edit_details(&id)?.disks[0].capacity, Some(1 << 30));
        let plan = b.removal_plan(&id)?;
        b.remove_with_files(&id, &plan)?;
        assert!(b.list()?.is_empty());
        assert_eq!(std::fs::read(&request.source).unwrap(), original);
    }
    println!("PASS imported RAW and QCOW2 expansion and owned-copy deletion preserve source files");
    Ok(())
}
