use lyra_vega_dbus::{Snapshot, SnapshotsClient};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::{CmdResult, State, dto, err, list};

dto!(Point from Snapshot { id: u32, timestamp: i64, trigger: String, description: String });

#[derive(Clone, Serialize)]
struct ClearProgress {
    deleted: u32,
    total: u32,
}

#[tauri::command]
pub async fn snapshots_available(state: State<'_>) -> CmdResult<bool> {
    state
        .dbus()
        .await?
        .snapshots()
        .available()
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_list(state: State<'_>) -> CmdResult<Vec<Point>> {
    state
        .dbus()
        .await?
        .snapshots()
        .list()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_create(state: State<'_>, description: String) -> CmdResult<u32> {
    state
        .dbus()
        .await?
        .snapshots()
        .create(&description)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_diff(state: State<'_>, id: u32) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .snapshots()
        .diff_packages(id)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_rollback(state: State<'_>, id: u32) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .snapshots()
        .rollback(id)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_delete(state: State<'_>, id: u32) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .snapshots()
        .delete(id)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn snapshots_set_retention(state: State<'_>, keep: u32) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .snapshots()
        .set_retention(keep)
        .await
        .map_err(err)
}

/// Apaga os pontos criados, emitindo `snapshots-clear-progress`.
#[tauri::command]
pub async fn snapshots_clear(app: AppHandle, state: State<'_>) -> CmdResult<u32> {
    let client = state.dbus().await?.snapshots();
    client
        .clear_with_progress(|deleted, total| {
            let _ = app.emit("snapshots-clear-progress", ClearProgress { deleted, total });
        })
        .await
        .map_err(err)
}
