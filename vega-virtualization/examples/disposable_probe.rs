//! Run ONLY in the existing marked disposable guest, as its unprivileged user.
use std::sync::atomic::AtomicBool;
use vega_virtualization::{Action, Backend, Connection, CreateRequest, State};

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
        iso: iso.clone(),
        cpus: 1,
        memory_mib: 512,
        disk_gib: 4,
    };
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
    println!(
        "PASS: cancellation, create, duplicate refusal, KVM start, pause/resume, force-stop, removal, source and disk preservation"
    );
    Ok(())
}
