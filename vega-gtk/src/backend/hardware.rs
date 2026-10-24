use lyra_vega_dbus::{HardwareClient, HardwareInventory};

use super::{CmdResult, State, dto, err};

dto!(Inventory from HardwareInventory { cpu: String, gpu: String, ram: String });

#[tauri::command]
pub async fn hardware_inventory(state: State<'_>) -> CmdResult<Inventory> {
    state
        .dbus()
        .await?
        .hardware()
        .inventory()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn hardware_firmware(state: State<'_>) -> CmdResult<String> {
    state
        .dbus()
        .await?
        .hardware()
        .firmware_status()
        .await
        .map_err(err)
}
