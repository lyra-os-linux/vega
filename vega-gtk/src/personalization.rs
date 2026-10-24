//! Personalização: tema, perfis do desktop (Sheliak), dock, menu, tela de
//! bloqueio e atalhos para os apps do GNOME. Tudo roda na sessão do
//! usuário (gsettings, shell-suite), sem o vegad.

use gio::prelude::*;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

const INTERFACE: &str = "org.gnome.desktop.interface";

fn interface_schema() -> Option<gio::Settings> {
    gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(INTERFACE, true))
        .map(|_| gio::Settings::new(INTERFACE))
}

fn interface_state(settings: &gio::Settings) -> Value {
    let accent = settings
        .settings_schema()
        .filter(|schema| schema.has_key("accent-color"))
        .map(|_| settings.string("accent-color").to_string())
        .unwrap_or_else(|| "blue".into());
    json!({ "color_scheme": settings.string("color-scheme").as_str(), "accent": accent })
}

/// Entrada única dos comandos de personalização: `op` escolhe a operação.
#[tauri::command]
pub async fn personalization_call(op: String, args: Value) -> Result<Value, String> {
    let _ = &args;
    match op.as_str() {
        // Esquema de cor e destaque do GNOME, para a janela acompanhar o desktop.
        "interface" => Ok(interface_schema()
            .map(|settings| interface_state(&settings))
            .unwrap_or_else(|| json!({ "color_scheme": "default", "accent": "blue" }))),
        _ => Err(format!("operação de personalização desconhecida: {op}")),
    }
}

/// Emite `interface-changed` quando o tema ou a cor de destaque do GNOME
/// mudam. Roda na thread principal, onde está o loop do GLib.
pub fn watch_accent(app: AppHandle) {
    let Some(settings) = interface_schema() else {
        return;
    };
    settings.connect_changed(None, move |settings, key| {
        if key == "color-scheme" || key == "accent-color" {
            let _ = app.emit("interface-changed", interface_state(settings));
        }
    });
    // A assinatura vive enquanto o processo viver.
    std::mem::forget(settings);
}
