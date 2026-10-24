use lyra_vega_dbus::{
    NvidiaStatus, PackageDetails, PackageRef, PreparationClient, PreparationKey, RepositoryRef,
    SoftwareClient, SoftwareEvent, UpdateStatus, ZbusSoftwareClient,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{CmdResult, State, dto, err, list};

dto!(Package from PackageRef {
    origin: String, id: String, name: String, description: String,
    installed: bool, icon: String, repository: String,
});
dto!(Repository from RepositoryRef { name: String, enabled: bool });
dto!(Details from PackageDetails {
    origin: String, id: String, name: String, description: String, installed: bool,
    installed_version: String, available_version: String, download_size: String,
    installed_size: String, dependencies: Vec<String>, licenses: Vec<String>,
    url: String, maintainer: String,
});
dto!(Status from UpdateStatus {
    checked_at: String, profile: String, native_count: u32, flatpak_count: u32,
    total_count: u32, security_count: u32, in_progress: bool, error: String,
});
dto!(Nvidia from NvidiaStatus {
    supported: bool, installed: bool, reboot_required: bool, gpu: String,
    secure_boot: String, state: String, detail: String, recovery_snapshot: u32,
});

async fn client(state: &State<'_>) -> CmdResult<ZbusSoftwareClient> {
    Ok(state.dbus().await?.software())
}

#[tauri::command]
pub async fn software_search(state: State<'_>, query: String) -> CmdResult<Vec<Package>> {
    client(&state)
        .await?
        .search(&query)
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn software_details(state: State<'_>, origin: String, id: String) -> CmdResult<Details> {
    client(&state)
        .await?
        .package_details(&origin, &id)
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn software_installed(state: State<'_>) -> CmdResult<Vec<Package>> {
    client(&state)
        .await?
        .list_installed()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn software_updates(state: State<'_>) -> CmdResult<Vec<Package>> {
    client(&state)
        .await?
        .list_updates()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn software_update_status(state: State<'_>) -> CmdResult<Status> {
    client(&state)
        .await?
        .update_status()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn software_request_check(state: State<'_>) -> CmdResult<Status> {
    client(&state)
        .await?
        .request_update_check()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn software_repos(state: State<'_>) -> CmdResult<Vec<Repository>> {
    client(&state)
        .await?
        .list_repos()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn software_set_repo_enabled(
    state: State<'_>,
    repo: String,
    enabled: bool,
) -> CmdResult<()> {
    client(&state)
        .await?
        .set_repo_enabled(&repo, enabled)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn software_nvidia_status(state: State<'_>) -> CmdResult<Nvidia> {
    client(&state)
        .await?
        .nvidia_status()
        .await
        .map(Into::into)
        .map_err(err)
}

/// Operações que viram transação no vegad.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    Install {
        origin: String,
        id: String,
    },
    Remove {
        origin: String,
        id: String,
    },
    UpdatePackage {
        origin: String,
        id: String,
    },
    UpdateAll,
    ClearCache,
    AddRepo {
        name: String,
        url: String,
    },
    TrustRepoKey {
        repo: String,
        key_id: String,
    },
    InstallNvidia,
    ApprovePreparationKey {
        repo: String,
        fingerprint: String,
        user_id: String,
        token: String,
    },
}

/// Evento de transação repassado à interface (`software-transaction`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TransactionEvent {
    Progress {
        id: u32,
        percent: u32,
        message: String,
    },
    Package {
        id: u32,
        package: String,
        phase: String,
        percent: u32,
    },
    Console {
        id: u32,
        source: String,
        line: String,
    },
    KeyPending {
        id: u32,
        repo: String,
        key_id: String,
        fingerprint: String,
        user_id: String,
    },
    Finished {
        id: u32,
        success: bool,
        message: String,
    },
}

/// Assina os sinais antes de iniciar a operação (como o vega-gtk) para não
/// perder o Finished de uma transação rápida, devolve o id e acompanha a
/// transação numa tarefa própria até o fim.
#[tauri::command]
pub async fn software_run(app: AppHandle, state: State<'_>, action: Action) -> CmdResult<u32> {
    let dbus = state.dbus().await?;
    let software = dbus.software();
    let events = software.subscribe().await.map_err(err)?;
    let id = match &action {
        Action::Install { origin, id } => software.install(origin, id).await,
        Action::Remove { origin, id } => software.remove(origin, id).await,
        Action::UpdatePackage { origin, id } => software.update_package(origin, id).await,
        Action::UpdateAll => software.update_all().await,
        Action::ClearCache => software.clear_cache().await,
        Action::AddRepo { name, url } => software.add_repo(name, url).await,
        Action::TrustRepoKey { repo, key_id } => software.trust_repo_key(repo, key_id).await,
        Action::InstallNvidia => software.install_nvidia(true).await,
        Action::ApprovePreparationKey {
            repo,
            fingerprint,
            user_id,
            token,
        } => {
            let key = PreparationKey {
                repo: repo.clone(),
                fingerprint: fingerprint.clone(),
                user_id: user_id.clone(),
                token: token.clone(),
            };
            return match dbus.preparation().approve_key(&key).await {
                Ok(id) => {
                    tauri::async_runtime::spawn(follow(app, events, id));
                    Ok(id)
                }
                Err(error) => Err(err(error)),
            };
        }
    }
    .map_err(err)?;
    tauri::async_runtime::spawn(follow(app, events, id));
    Ok(id)
}

async fn follow(app: AppHandle, mut events: lyra_vega_dbus::SoftwareEventStream, id: u32) {
    loop {
        let event = match events.next_transaction(id).await {
            Ok(SoftwareEvent::Progress(p)) if p.transaction_id == id => {
                TransactionEvent::Progress {
                    id,
                    percent: p.percent,
                    message: p.message,
                }
            }
            Ok(SoftwareEvent::PackageProgress(p)) if p.transaction_id == id => {
                TransactionEvent::Package {
                    id,
                    package: p.package,
                    phase: p.phase,
                    percent: p.percent,
                }
            }
            Ok(SoftwareEvent::ConsoleLine(c)) if c.transaction_id == id => {
                TransactionEvent::Console {
                    id,
                    source: c.source,
                    line: c.line,
                }
            }
            Ok(SoftwareEvent::KeyPending(k)) if k.transaction_id == id => {
                TransactionEvent::KeyPending {
                    id,
                    repo: k.repo,
                    key_id: k.key_id,
                    fingerprint: k.fingerprint,
                    user_id: k.user_id,
                }
            }
            Ok(SoftwareEvent::Finished(f)) if f.transaction_id == id => {
                let _ = app.emit(
                    "software-transaction",
                    TransactionEvent::Finished {
                        id,
                        success: f.success,
                        message: f.message,
                    },
                );
                return;
            }
            Ok(_) => continue,
            Err(error) => {
                let _ = app.emit(
                    "software-transaction",
                    TransactionEvent::Finished {
                        id,
                        success: false,
                        message: err(error),
                    },
                );
                return;
            }
        };
        let _ = app.emit("software-transaction", event);
    }
}

/// Mantém o contador de atualizações em dia (`updates-available`), com
/// reconexão a cada 5 s, como `watch_dashboard_updates` do vega-gtk.
pub fn watch_updates(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(dbus) = lyra_vega_dbus::VegaDbus::connect().await
                && let Ok(mut events) = dbus.software().subscribe().await
            {
                while let Ok(event) = events.next().await {
                    if let SoftwareEvent::UpdatesAvailable(count) = event {
                        let _ = app.emit("updates-available", count);
                    }
                }
            }
            async_io::Timer::after(std::time::Duration::from_secs(5)).await;
        }
    });
}
