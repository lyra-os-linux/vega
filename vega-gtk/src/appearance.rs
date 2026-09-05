use gtk::{gio, gio::prelude::*};

const SCHEMA: &str = "org.gnome.desktop.interface";

/// Tema claro/escuro do GNOME inteiro (`color-scheme`), o mesmo valor lido
/// pelo GNOME Shell, Nautilus e qualquer app libadwaita — igual ao painel
/// Aparência das Configurações do GNOME.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// `org.gnome.desktop.interface` é do GNOME, não do vegad — mesma lógica de
/// schema_available() do screensaver, sem depender do backend.
pub fn schema_available() -> bool {
    gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SCHEMA, true))
        .is_some()
}

pub fn current_theme() -> Theme {
    if !schema_available() {
        return Theme::default();
    }
    match gio::Settings::new(SCHEMA).string("color-scheme").as_str() {
        "prefer-dark" => Theme::Dark,
        "prefer-light" => Theme::Light,
        _ => Theme::System,
    }
}

pub fn apply_theme(theme: Theme) {
    if !schema_available() {
        return;
    }
    let value = match theme {
        Theme::System => "default",
        Theme::Light => "prefer-light",
        Theme::Dark => "prefer-dark",
    };
    let _ = gio::Settings::new(SCHEMA).set_string("color-scheme", value);
}
