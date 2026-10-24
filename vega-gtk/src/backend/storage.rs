use lyra_vega_dbus::{StorageClient, StorageVolume};

use super::{CmdResult, State, dto, err, list};

dto!(Volume from StorageVolume {
    name: String, path: String, kind: String, fs_type: String, size: String, used: String,
    available: String, use_percent: u32, mountpoint: String, model: String, removable: bool,
    can_mount: bool, can_unmount: bool,
});

#[tauri::command]
pub async fn storage_list(state: State<'_>) -> CmdResult<Vec<Volume>> {
    state
        .dbus()
        .await?
        .storage()
        .list()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn storage_mount(state: State<'_>, path: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .storage()
        .mount(&path)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn storage_unmount(state: State<'_>, path: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .storage()
        .unmount(&path)
        .await
        .map_err(err)
}
