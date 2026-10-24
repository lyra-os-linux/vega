use lyra_vega_dbus::{DateTimeClient, DateTimeStatus};
use serde::Serialize;

use super::{CmdResult, State, dto, err};

dto!(Status from DateTimeStatus { timezone: String, ntp: bool, locale: String, keymap: String });

#[derive(Serialize)]
pub struct Choices {
    status: Status,
    timezones: Vec<String>,
    locales: Vec<String>,
    keymaps: Vec<String>,
}

/// Status e as três listas; como no vega-gtk, todas precisam dar certo.
#[tauri::command]
pub async fn datetime_load(state: State<'_>) -> CmdResult<Choices> {
    let client = state.dbus().await?.datetime();
    let (status, timezones, locales, keymaps) = futures_util::join!(
        client.status(),
        client.list_timezones(),
        client.list_locales(),
        client.list_keymaps()
    );
    Ok(Choices {
        status: status.map_err(err)?.into(),
        timezones: timezones.map_err(err)?,
        locales: locales.map_err(err)?,
        keymaps: keymaps.map_err(err)?,
    })
}

#[tauri::command]
pub async fn datetime_apply(
    state: State<'_>,
    timezone: String,
    ntp: bool,
    locale: String,
    keymap: String,
) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .datetime()
        .apply(&timezone, ntp, &locale, &keymap)
        .await
        .map_err(err)
}
