use lyra_vega_dbus::{UserInfo, UsersClient};
use serde::Deserialize;

use super::{CmdResult, State, dto, err, list};

dto!(User from UserInfo { username: String, full_name: String, groups: Vec<String>, is_admin: bool });

/// Formulário de criação/edição. `photo` são os bytes já lidos no backend a
/// partir do caminho escolhido (PNG/JPEG até 5 MB); vazio mantém a atual.
#[derive(Deserialize)]
pub struct Form {
    username: String,
    full_name: String,
    password: String,
    groups: Vec<String>,
    photo_path: String,
    is_admin: bool,
}

const PHOTO_LIMIT: u64 = 5 * 1024 * 1024;

fn read_photo(path: &str) -> CmdResult<Vec<u8>> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    let lower = path.to_ascii_lowercase();
    if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
        return Err(gettextrs::gettext("A foto deve ser PNG ou JPEG."));
    }
    let size = std::fs::metadata(path).map_err(err)?.len();
    if size > PHOTO_LIMIT {
        return Err(gettextrs::gettext("A foto deve ter no máximo 5 MB."));
    }
    std::fs::read(path).map_err(err)
}

#[tauri::command]
pub async fn users_list(state: State<'_>) -> CmdResult<Vec<User>> {
    state
        .dbus()
        .await?
        .users()
        .list()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn users_save(state: State<'_>, form: Form, create: bool) -> CmdResult<()> {
    let photo = read_photo(&form.photo_path)?;
    let client = state.dbus().await?.users();
    if create {
        client
            .create(
                &form.username,
                &form.full_name,
                &form.password,
                &form.groups,
                &photo,
                form.is_admin,
            )
            .await
    } else {
        client
            .update(
                &form.username,
                &form.full_name,
                &form.password,
                &form.groups,
                &photo,
                form.is_admin,
            )
            .await
    }
    .map_err(err)
}

#[tauri::command]
pub async fn users_set_admin(state: State<'_>, username: String, admin: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .users()
        .set_admin(&username, admin)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn users_remove(state: State<'_>, username: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .users()
        .remove(&username)
        .await
        .map_err(err)
}
