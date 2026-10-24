use lyra_vega_dbus::{PreparationClient, PreparationKey, PreparationStatus};

use super::{CmdResult, State, dto, err, list};

dto!(Status from PreparationStatus {
    state: String, phase: String, error_kind: String, last_error: String,
    updated_at: String, next_retry_at: String, can_retry: bool,
});
dto!(Key from PreparationKey { repo: String, fingerprint: String, user_id: String, token: String });

#[tauri::command]
pub async fn preparation_status(state: State<'_>) -> CmdResult<Option<Status>> {
    state
        .dbus()
        .await?
        .preparation()
        .status()
        .await
        .map(|s| s.map(Into::into))
        .map_err(err)
}

#[tauri::command]
pub async fn preparation_pending_keys(state: State<'_>) -> CmdResult<Vec<Key>> {
    state
        .dbus()
        .await?
        .preparation()
        .pending_keys()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn preparation_retry(state: State<'_>) -> CmdResult<()> {
    state.dbus().await?.preparation().retry().await.map_err(err)
}
