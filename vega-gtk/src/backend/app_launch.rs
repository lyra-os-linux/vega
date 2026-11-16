//! Abre, a partir de um cartão da Loja, um app com tela própria. Só os IDs
//! da lista abaixo: a interface não escolhe o programa.

use gio::prelude::*;

/// IDs de .desktop que um cartão pode abrir (`launch` no catálogo).
const ALLOWED: [&str; 1] = ["org.lyraos.LyraDriver"];

#[tauri::command]
pub fn app_launch(id: String) -> Result<(), String> {
    if !ALLOWED.contains(&id.as_str()) {
        return Err(format!("app não permitido: {id}"));
    }
    // Pelo .desktop instalado, como o menu de apps: sem depender do
    // gtk-launch (gtk3-tools) e sem deixar processo filho para trás.
    let desktop = format!("{id}.desktop");
    let app = gio::AppInfo::all()
        .into_iter()
        .find(|app| app.id().as_deref() == Some(desktop.as_str()))
        .ok_or_else(|| format!("{id} não está instalado"))?;
    app.launch(&[], gio::AppLaunchContext::NONE)
        .map_err(|e| format!("não foi possível abrir {id}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_ids_outside_the_list() {
        assert!(app_launch("org.gnome.Terminal".into()).is_err());
        assert!(app_launch("../../bin/sh".into()).is_err());
    }
}
