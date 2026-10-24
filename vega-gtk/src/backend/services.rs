use lyra_vega_dbus::{ManagedService, ServicesClient};

use super::{CmdResult, State, dto, err, list};

dto!(Service from ManagedService {
    name: String, label: String, description: String, enabled: bool, active: bool, available: bool,
});

#[tauri::command]
pub async fn services_list(state: State<'_>, all: bool) -> CmdResult<Vec<Service>> {
    let client = state.dbus().await?.services();
    if all {
        client.list_all().await
    } else {
        client.list().await
    }
    .map(list)
    .map_err(err)
}

#[tauri::command]
pub async fn services_set_enabled(state: State<'_>, name: String, enabled: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .services()
        .set_enabled(&name, enabled)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn services_set_running(state: State<'_>, name: String, running: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .services()
        .set_running(&name, running)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn services_restart(state: State<'_>, name: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .services()
        .restart(&name)
        .await
        .map_err(err)
}
