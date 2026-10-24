use lyra_vega_dbus::{BackendStatus, DaemonMetadata, MetadataClient, SystemClient};
use serde::Serialize;

use super::{CmdResult, State, dto, err};

dto!(Status from BackendStatus { version: String, distro: String, logo_path: String });
dto!(Metadata from DaemonMetadata { profile: String, version: String, capabilities: Vec<String> });

#[derive(Serialize)]
pub struct DiskUsage {
    used: String,
    total: String,
    percent: u32,
}

#[tauri::command]
pub async fn system_status(state: State<'_>) -> CmdResult<Status> {
    state
        .dbus()
        .await?
        .system()
        .status()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn system_disk_usage(state: State<'_>) -> CmdResult<DiskUsage> {
    let (used, total, percent) = state
        .dbus()
        .await?
        .system()
        .disk_usage()
        .await
        .map_err(err)?;
    Ok(DiskUsage {
        used,
        total,
        percent,
    })
}

#[tauri::command]
pub async fn system_metadata(state: State<'_>) -> CmdResult<Metadata> {
    state
        .dbus()
        .await?
        .metadata()
        .metadata()
        .await
        .map(Into::into)
        .map_err(err)
}
