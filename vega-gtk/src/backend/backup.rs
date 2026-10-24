use lyra_vega_dbus::{BackupClient, BackupConfig, BackupEvent, BackupSnapshot};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{CmdResult, State, dto, err, list};

dto!(Config from BackupConfig {
    id: String, paths: Vec<String>, destination: String, destination_uuid: String, frequency: String,
});
dto!(Snapshot from BackupSnapshot { id: String, timestamp: i64, file_count: u64, size_bytes: u64 });

impl From<Config> for BackupConfig {
    fn from(c: Config) -> Self {
        Self {
            id: c.id,
            paths: c.paths,
            destination: c.destination,
            destination_uuid: c.destination_uuid,
            frequency: c.frequency,
        }
    }
}

#[tauri::command]
pub async fn backup_configs(state: State<'_>) -> CmdResult<Vec<Config>> {
    state
        .dbus()
        .await?
        .backup()
        .list_configs()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn backup_create_config(state: State<'_>, config: Config) -> CmdResult<String> {
    state
        .dbus()
        .await?
        .backup()
        .create_config(config.into())
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn backup_delete_config(state: State<'_>, id: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .backup()
        .delete_config(&id)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn backup_snapshots(state: State<'_>, config: String) -> CmdResult<Vec<Snapshot>> {
    state
        .dbus()
        .await?
        .backup()
        .list_snapshots(&config)
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn backup_snapshot_paths(
    state: State<'_>,
    config: String,
    snapshot: String,
) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .backup()
        .list_snapshot_paths(&config, &snapshot)
        .await
        .map_err(err)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    RunNow {
        config: String,
    },
    RestoreItems {
        snapshot: String,
        target: String,
        mode: String,
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Event {
    Progress {
        id: u32,
        percent: u32,
        message: String,
    },
    Finished {
        id: u32,
        success: bool,
        message: String,
    },
    Alert {
        config: String,
        failures: u32,
        message: String,
    },
}

/// Inicia backup ou restauração e repassa `backup-transaction` até o fim.
#[tauri::command]
pub async fn backup_run(app: AppHandle, state: State<'_>, action: Action) -> CmdResult<u32> {
    let client = state.dbus().await?.backup();
    let mut events = client.subscribe().await.map_err(err)?;
    let id = match &action {
        Action::RunNow { config } => client.run_now(config).await,
        Action::RestoreItems {
            snapshot,
            target,
            mode,
            paths,
        } => client.restore_items(snapshot, target, mode, paths).await,
    }
    .map_err(err)?;
    tauri::async_runtime::spawn(async move {
        loop {
            let event = match events.next().await {
                Ok(BackupEvent::BackupProgress(p) | BackupEvent::RestoreProgress(p))
                    if p.transaction_id == id =>
                {
                    Event::Progress {
                        id,
                        percent: p.percent,
                        message: p.message,
                    }
                }
                Ok(BackupEvent::BackupFinished(f) | BackupEvent::RestoreFinished(f))
                    if f.transaction_id == id =>
                {
                    let _ = app.emit(
                        "backup-transaction",
                        Event::Finished {
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
                        "backup-transaction",
                        Event::Finished {
                            id,
                            success: false,
                            message: err(error),
                        },
                    );
                    return;
                }
            };
            let _ = app.emit("backup-transaction", event);
        }
    });
    Ok(id)
}

/// Alertas de falhas consecutivas, a qualquer momento (`backup-alert`).
pub fn watch_alerts(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(dbus) = lyra_vega_dbus::VegaDbus::connect().await
                && let Ok(mut events) = dbus.backup().subscribe().await
            {
                while let Ok(event) = events.next().await {
                    if let BackupEvent::Alert(a) = event {
                        let _ = app.emit(
                            "backup-alert",
                            Event::Alert {
                                config: a.config_id,
                                failures: a.consecutive_failures,
                                message: a.message,
                            },
                        );
                    }
                }
            }
            async_io::Timer::after(std::time::Duration::from_secs(5)).await;
        }
    });
}
