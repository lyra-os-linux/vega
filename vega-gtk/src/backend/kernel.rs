use lyra_vega_dbus::{BootStatus, KernelClient};

use super::{CmdResult, State, dto, err};

dto!(Boot from BootStatus { loader: String, default_entry: String, timeout: u32, cmdline: String });

#[tauri::command]
pub async fn kernel_installed(state: State<'_>) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .kernel()
        .list_installed()
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn kernel_available(state: State<'_>) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .kernel()
        .available_packages()
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn kernel_boot_status(state: State<'_>) -> CmdResult<Boot> {
    state
        .dbus()
        .await?
        .kernel()
        .boot_status()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn kernel_boot_entries(state: State<'_>) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .kernel()
        .list_boot_entries()
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn kernel_apply_boot(
    state: State<'_>,
    entry: String,
    timeout: u32,
    cmdline: String,
) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .kernel()
        .apply_boot_config(&entry, timeout, &cmdline)
        .await
        .map_err(err)
}
