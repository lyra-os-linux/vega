use lyra_vega_dbus::LogsClient;

use super::{CmdResult, State, err};

#[tauri::command]
pub async fn logs_units(state: State<'_>) -> CmdResult<Vec<String>> {
    state.dbus().await?.logs().list_units().await.map_err(err)
}

#[tauri::command]
pub async fn logs_query(
    state: State<'_>,
    unit: String,
    priority: String,
    since: String,
    search: String,
    limit: u32,
) -> CmdResult<Vec<String>> {
    state
        .dbus()
        .await?
        .logs()
        .query(&unit, &priority, &since, search.trim(), limit)
        .await
        .map_err(err)
}
