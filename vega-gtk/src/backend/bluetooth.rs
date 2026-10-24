use lyra_vega_dbus::{BluetoothClient, BluetoothDevice, BluetoothStatus};

use super::{CmdResult, State, dto, err, list};

dto!(Status from BluetoothStatus {
    available: bool, powered: bool, discoverable: bool, pairable: bool, scanning: bool,
    controller: String, controller_name: String, transfer_available: bool,
    receiver_active: bool, receive_path: String,
});
dto!(Device from BluetoothDevice {
    address: String, name: String, alias: String, icon: String, paired: bool,
    trusted: bool, connected: bool, blocked: bool, rssi: i32,
});

#[tauri::command]
pub async fn bluetooth_status(state: State<'_>) -> CmdResult<Status> {
    state
        .dbus()
        .await?
        .bluetooth()
        .status()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn bluetooth_devices(state: State<'_>) -> CmdResult<Vec<Device>> {
    state
        .dbus()
        .await?
        .bluetooth()
        .devices()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn bluetooth_set_powered(state: State<'_>, on: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .bluetooth()
        .set_powered(on)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn bluetooth_set_scanning(state: State<'_>, on: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .bluetooth()
        .set_scanning(on)
        .await
        .map_err(err)
}

/// action: "pair" | "connect" | "disconnect"
#[tauri::command]
pub async fn bluetooth_device_action(
    state: State<'_>,
    address: String,
    action: String,
) -> CmdResult<()> {
    let client = state.dbus().await?.bluetooth();
    match action.as_str() {
        "pair" => client.pair(&address).await,
        "connect" => client.connect(&address).await,
        "disconnect" => client.disconnect(&address).await,
        other => return Err(format!("ação Bluetooth desconhecida: {other}")),
    }
    .map_err(err)
}

#[tauri::command]
pub async fn bluetooth_send_file(state: State<'_>, address: String, path: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .bluetooth()
        .send_file(&address, &path)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn bluetooth_start_receiver(state: State<'_>, directory: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .bluetooth()
        .start_receiver(&directory)
        .await
        .map_err(err)
}
