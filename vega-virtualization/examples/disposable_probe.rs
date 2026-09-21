//! Run ONLY in the existing marked disposable guest, as its unprivileged user.
use std::sync::atomic::AtomicBool;
use vega_virtualization::{Action, Backend, Connection, CreateRequest, Firmware, MediaKind, State};

fn main() -> Result<(), String> {
    let cmdline = std::fs::read_to_string("/proc/cmdline").map_err(|e| e.to_string())?;
    if !cmdline
        .split_whitespace()
        .any(|s| s == "lyra.virtualization-test=1")
    {
        return Err("This probe only runs inside the marked disposable VM.".into());
    }
    let backend = Backend::open(Connection::Personal, false)?;
    if !backend.list()?.is_empty() {
        return Err("Fixture session must initially be empty.".into());
    }
    let iso = std::path::PathBuf::from("/tmp/lyra-vms-fixture.iso");
    let original = std::fs::read(&iso).map_err(|e| e.to_string())?;
    let request = CreateRequest {
        name: "Lyra disposable probe".into(),
        source: iso.clone(),
        kind: MediaKind::Iso,
        firmware: Firmware::Bios,
        cpus: 1,
        memory_mib: 512,
        disk_gib: 4,
    };
    if std::env::args().any(|arg| arg == "--console-fixture") {
        let request = CreateRequest {
            name: "Lyra console test".into(),
            source: "/tmp/lyra-console-boot.raw".into(),
            kind: MediaKind::Raw,
            firmware: Firmware::Bios,
            ..request
        };
        let uuid = backend.create(&request, &AtomicBool::new(false))?;
        backend.act(&uuid, Action::Start)?;
        std::fs::write("/tmp/lyra-console-uuid", &uuid).map_err(|e| e.to_string())?;
        println!("CONSOLE {uuid}");
        return Ok(());
    }
    assert!(backend.create(&request, &AtomicBool::new(true)).is_err());
    assert!(backend.list()?.is_empty());
    let uuid = backend.create(&request, &AtomicBool::new(false))?;
    println!("CREATED {uuid}");
    assert_eq!(backend.list()?[0].state, State::Off);
    backend.configure(&uuid, 1, 768)?;
    assert_eq!(backend.list()?[0].memory_kib, 768 * 1024);
    backend.configure(&uuid, 1, 512)?;
    assert!(backend.create(&request, &AtomicBool::new(false)).is_err());
    backend.act(&uuid, Action::Start)?;
    assert_eq!(backend.list()?[0].state, State::Running);
    assert!(backend.configure(&uuid, 1, 768).is_err());
    assert!(backend.act(&uuid, Action::Remove).is_err());
    backend.act(&uuid, Action::Pause)?;
    assert_eq!(backend.list()?[0].state, State::Paused);
    backend.act(&uuid, Action::Resume)?;
    backend.act(&uuid, Action::ForceStop)?;
    backend.act(&uuid, Action::Remove)?;
    assert!(backend.list()?.is_empty());
    assert_eq!(std::fs::read(&iso).map_err(|e| e.to_string())?, original);
    let root =
        std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share/lyra-vms");
    assert!(root.join(format!("{uuid}.qcow2")).is_file());
    assert!(root.join(format!("{uuid}.iso")).is_file());
    for (kind, source, firmware) in [
        (
            MediaKind::Qcow2,
            "/tmp/lyra-vms-fixture.qcow2",
            Firmware::Bios,
        ),
        (MediaKind::Raw, "/tmp/lyra-vms-fixture.raw", Firmware::Bios),
        (MediaKind::Iso, "/tmp/lyra-vms-fixture.iso", Firmware::Uefi),
    ] {
        let original = std::fs::read(source).map_err(|e| e.to_string())?;
        let import = CreateRequest {
            name: format!("Import {kind:?} {firmware:?}"),
            source: source.into(),
            kind,
            firmware,
            ..request.clone()
        };
        let id = backend.create(&import, &AtomicBool::new(false))?;
        backend.act(&id, Action::Start)?;
        assert_eq!(backend.list()?[0].state, State::Running);
        let conn = virt::connect::Connect::open(Some("qemu:///session")).unwrap();
        let domain = virt::domain::Domain::lookup_by_uuid_string(&conn, &id).unwrap();
        let xml = domain
            .get_xml_desc(virt::sys::VIR_DOMAIN_XML_INACTIVE)
            .unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let nvram = doc
            .descendants()
            .find(|n| n.has_tag_name("nvram"))
            .and_then(|n| n.text())
            .map(str::to_string);
        if firmware == Firmware::Uefi {
            assert!(nvram.is_some(), "UEFI must have persistent NVRAM");
        }
        backend.act(&id, Action::ForceStop)?;
        backend.act(&id, Action::Remove)?;
        if let Some(path) = nvram {
            assert!(std::path::Path::new(&path).is_file(), "NVRAM preserved");
        }
        assert_eq!(std::fs::read(source).unwrap(), original);
        if kind != MediaKind::Iso {
            assert_eq!(
                std::fs::read(root.join(format!("{id}.{}", kind.disk_format()))).unwrap(),
                original
            );
        }
        println!("PASS import {kind:?} {firmware:?} {id}");
    }
    let volume_names = || {
        let conn = virt::connect::Connect::open(Some("qemu:///session")).unwrap();
        let pool = virt::storage_pool::StoragePool::lookup_by_name(&conn, "lyra-vms").unwrap();
        let mut names: Vec<_> = pool
            .list_all_volumes(0)
            .unwrap()
            .iter()
            .map(|v| v.get_name().unwrap())
            .collect();
        names.sort();
        names
    };
    let before = volume_names();
    let cancel = AtomicBool::new(false);
    assert!(
        backend
            .create_with_progress(&request, &cancel, |_, _| {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            })
            .is_err()
    );
    assert_eq!(
        volume_names(),
        before,
        "mid-copy cancellation rolled back only new volumes"
    );
    let broken_source = std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
        .join("probe-truncated-source.iso");
    std::fs::copy(&iso, &broken_source).map_err(|e| e.to_string())?;
    let broken = CreateRequest {
        source: broken_source.clone(),
        ..request.clone()
    };
    assert!(
        backend
            .create_with_progress(&broken, &AtomicBool::new(false), |_, _| {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&broken_source)
                    .unwrap()
                    .set_len(0)
                    .unwrap();
            })
            .is_err()
    );
    assert_eq!(
        volume_names(),
        before,
        "read failure rolled back only new volumes"
    );
    std::fs::remove_file(broken_source).map_err(|e| e.to_string())?;
    assert!(backend.list()?.is_empty());
    println!("PASS mid-copy cancellation and truncated-source rollback");
    println!(
        "PASS: cancellation, create, duplicate refusal, KVM start, pause/resume, force-stop, removal, source and disk preservation"
    );
    Ok(())
}
