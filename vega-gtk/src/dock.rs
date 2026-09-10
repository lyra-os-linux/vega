use std::path::PathBuf;

use crate::i18n::gettext;
use gtk::{gio, gio::prelude::*, glib};

const EXTENSION_UUID: &str = "sheliak@lyraos.com.br";
const SCHEMA_ID: &str = "org.gnome.shell.extensions.sheliak";
const SCHEMA_PATH: &str = "/org/gnome/shell/extensions/sheliak/";
const SHELL_SCHEMA_ID: &str = "org.gnome.shell";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSettings {
    pub position: String,
    pub hide_mode: String,
    pub hide_delay_ms: u32,
    pub icon_size: u32,
    pub edge_margin: u32,
    pub animation: bool,
    pub minimize_animation: String,
    pub extend_to_edges: bool,
    pub content_alignment: String,
    pub show_running: bool,
    pub running_apps_position: String,
    pub show_trash: bool,
    pub show_apps_button: bool,
    pub fullscreen_hide: bool,
}

/// Menus da barra superior (Aplicativos/Locais) e seu conteúdo — mesma
/// extensão Sheliak do dock, mas editados na aba "Menu" da Personalização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSettings {
    pub panel_height: u32,
    pub floating_panel: bool,
    pub panel_margin: u32,
    pub show_clock: bool,
    pub show_panel_indicators: bool,
    pub show_applications_menu: bool,
    pub show_places_menu: bool,
    pub show_network_menu: bool,
    pub show_system_menu: bool,
    pub show_system_about: bool,
    pub show_search_menu: bool,
    pub hide_workspace_button: bool,
    pub panel_menu_position: String,
    pub show_application_icons: bool,
    pub sort_applications_menu: bool,
    pub open_application_submenus_sideways: bool,
    pub show_place_bookmarks: bool,
    pub show_place_volumes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockError(String);

impl std::fmt::Display for DockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DockError {}

/// Ao contrário do screensaver, o schema do Sheliak não fica no
/// diretório global do glib-2.0: como toda extensão GNOME Shell, ele é
/// empacotado dentro do próprio diretório da extensão. Por isso precisamos
/// achar essa pasta e carregar o schema explicitamente dali, em vez de usar
/// `SettingsSchemaSource::default()`.
fn extension_dir() -> Option<PathBuf> {
    let mut candidates = vec![
        glib::user_data_dir()
            .join("gnome-shell/extensions")
            .join(EXTENSION_UUID),
    ];
    for base in ["/usr/share", "/usr/local/share"] {
        candidates.push(
            PathBuf::from(base)
                .join("gnome-shell/extensions")
                .join(EXTENSION_UUID),
        );
    }
    candidates
        .into_iter()
        .find(|dir| dir.join("metadata.json").is_file())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopProfile {
    Lyra,
    Ubuntu,
    Windows10,
    Windows11,
    GnomeVanilla,
}

pub fn current_profile() -> DesktopProfile {
    if !is_enabled() {
        return DesktopProfile::GnomeVanilla;
    }
    open_settings().map_or(DesktopProfile::Lyra, |settings| {
        match string_or(&settings, "desktop-profile", "lyra").as_str() {
            "ubuntu" => DesktopProfile::Ubuntu,
            "windows10" => DesktopProfile::Windows10,
            "windows11" => DesktopProfile::Windows11,
            _ => DesktopProfile::Lyra,
        }
    })
}

const PROFILE_MENUS: [&str; 4] = [
    "show-applications-menu",
    "show-places-menu",
    "show-system-menu",
    "show-search-menu",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SavedLyraProfile {
    position: String,
    extended: bool,
    floating: bool,
    extended_alignment: String,
    menus: [bool; 4],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SavedDesktopProfile {
    layout: SavedLyraProfile,
    hide_mode: String,
    icon_size: u32,
    show_trash: bool,
    show_apps: bool,
    show_clock: bool,
    show_indicators: bool,
    fullscreen_hide: bool,
}

impl SavedDesktopProfile {
    fn capture(settings: &gio::Settings) -> Self {
        Self {
            layout: SavedLyraProfile::capture(settings),
            hide_mode: settings.string("hide-mode").into(),
            icon_size: settings.uint("icon-size"),
            show_trash: settings.boolean("show-trash"),
            show_apps: settings.boolean("show-apps-button"),
            show_clock: settings.boolean("show-clock"),
            show_indicators: settings.boolean("show-panel-indicators"),
            fullscreen_hide: settings.boolean("fullscreen-hide"),
        }
    }

    fn restore(&self, settings: &gio::Settings) -> Result<(), glib::BoolError> {
        self.layout.restore(settings)?;
        settings.set_string("hide-mode", &self.hide_mode)?;
        settings.set_uint("icon-size", self.icon_size)?;
        for (key, value) in [
            ("show-trash", self.show_trash),
            ("show-apps-button", self.show_apps),
            ("show-clock", self.show_clock),
            ("show-panel-indicators", self.show_indicators),
            ("fullscreen-hide", self.fullscreen_hide),
        ] {
            settings.set_boolean(key, value)?;
        }
        Ok(())
    }
}

fn apply_saved_desktop_profile(
    settings: &gio::Settings,
    profile: DesktopProfile,
) -> Result<(), DockError> {
    use std::collections::BTreeMap;
    let key = match profile {
        DesktopProfile::Lyra => "lyra",
        DesktopProfile::Ubuntu => "ubuntu",
        DesktopProfile::Windows10 => "windows10",
        DesktopProfile::Windows11 => "windows11",
        DesktopProfile::GnomeVanilla => return Ok(()),
    };
    let current = settings.string("desktop-profile");
    if current == key {
        return Ok(());
    }
    let error = || {
        DockError(gettext(
            "Não foi possível restaurar as preferências dos perfis.",
        ))
    };
    if !["lyra", "ubuntu", "windows10", "windows11"].contains(&current.as_str()) {
        return Err(error());
    }
    let json = settings.string("desktop-profile-settings");
    let mut saved: BTreeMap<String, SavedDesktopProfile> = if json.is_empty() {
        BTreeMap::new()
    } else {
        serde_json::from_str(&json).map_err(|_| error())?
    };
    if saved
        .keys()
        .any(|name| !["lyra", "ubuntu", "windows10", "windows11"].contains(&name.as_str()))
    {
        return Err(error());
    }
    let before = SavedDesktopProfile::capture(settings);
    // Migrate the Lyra snapshot created by the Ubuntu-only Vega version.
    if current == "ubuntu" && !saved.contains_key("lyra") {
        let layout: SavedLyraProfile =
            serde_json::from_str(&settings.string("lyra-profile-settings")).map_err(|_| error())?;
        let mut lyra = before.clone();
        lyra.layout = layout;
        saved.insert("lyra".into(), lyra);
    }
    saved.insert(current.to_string(), before.clone());
    let target = saved.get(key).cloned().unwrap_or_else(|| {
        let mut preset = saved.get("lyra").cloned().unwrap_or_else(|| before.clone());
        preset.layout.menus = [false; 4];
        preset.layout.extended = true;
        preset.layout.extended_alignment = if profile == DesktopProfile::Windows11 {
            "center"
        } else {
            "start"
        }
        .into();
        preset.layout.position = if profile == DesktopProfile::Ubuntu {
            "left"
        } else {
            "bottom"
        }
        .into();
        preset.layout.floating = profile == DesktopProfile::Ubuntu;
        if matches!(
            profile,
            DesktopProfile::Windows10 | DesktopProfile::Windows11
        ) {
            preset.hide_mode = "always".into();
            preset.icon_size = if profile == DesktopProfile::Windows10 {
                32
            } else {
                28
            };
            preset.show_trash = false;
            preset.show_apps = true;
            preset.show_clock = true;
            preset.show_indicators = true;
            preset.fullscreen_hide = false;
        }
        preset
    });
    let json = serde_json::to_string(&saved).map_err(|_| error())?;
    settings.delay();
    let result = (|| {
        target.restore(settings)?;
        settings.set_string("desktop-profile-settings", &json)?;
        if current == "lyra" {
            let legacy =
                serde_json::to_string(&before.layout).expect("typed layout is serializable");
            settings.set_string("lyra-profile-settings", &legacy)?;
        }
        settings.set_string("desktop-profile", key)?;
        Ok::<_, glib::BoolError>(())
    })();
    if result.is_err() {
        settings.revert();
        return Err(error());
    }
    settings.apply();
    Ok(())
}

impl SavedLyraProfile {
    fn capture(settings: &gio::Settings) -> Self {
        Self {
            position: settings.string("position").into(),
            extended: settings.boolean("extend-to-edges"),
            floating: settings.boolean("floating-panel"),
            extended_alignment: settings.string("extended-content-alignment").into(),
            menus: PROFILE_MENUS.map(|key| settings.boolean(key)),
        }
    }
    fn restore(&self, settings: &gio::Settings) -> Result<(), glib::BoolError> {
        settings.set_string("position", &self.position)?;
        settings.set_boolean("extend-to-edges", self.extended)?;
        settings.set_boolean("floating-panel", self.floating)?;
        settings.set_string("extended-content-alignment", &self.extended_alignment)?;
        for (key, visible) in PROFILE_MENUS.iter().zip(self.menus) {
            settings.set_boolean(key, visible)?;
        }
        Ok(())
    }
}

pub fn apply_profile(profile: DesktopProfile) -> Result<(), DockError> {
    if profile == DesktopProfile::GnomeVanilla {
        return set_enabled(false);
    }
    let settings = open_settings().ok_or_else(|| {
        DockError(gettext(
            "A extensão Sheliak não está instalada ou não pôde ser encontrada.",
        ))
    })?;
    apply_profile_settings(&settings, profile)?;
    set_enabled(true)
}

fn apply_profile_settings(
    settings: &gio::Settings,
    profile: DesktopProfile,
) -> Result<(), DockError> {
    if has_key(settings, "desktop-profile-settings") {
        return apply_saved_desktop_profile(settings, profile);
    }
    if matches!(
        profile,
        DesktopProfile::Windows10 | DesktopProfile::Windows11
    ) {
        return Err(DockError(gettext(
            "Atualize o Sheliak para usar os perfis Windows.",
        )));
    }
    if ![
        "extended-content-alignment",
        "desktop-profile",
        "lyra-profile-settings",
    ]
    .iter()
    .all(|key| has_key(settings, key))
    {
        if profile == DesktopProfile::Lyra {
            return Ok(());
        }
        return Err(DockError(gettext(
            "Atualize o Sheliak para usar o perfil Ubuntu.",
        )));
    }
    let was_ubuntu = settings.string("desktop-profile") == "ubuntu";
    let saved = if was_ubuntu && profile == DesktopProfile::Lyra {
        Some(
            serde_json::from_str::<SavedLyraProfile>(&settings.string("lyra-profile-settings"))
                .map_err(|_| {
                    DockError(gettext(
                        "Não foi possível restaurar as preferências do perfil Lyra.",
                    ))
                })?,
        )
    } else {
        None
    };
    let snapshot = if !was_ubuntu && profile == DesktopProfile::Ubuntu {
        Some(
            serde_json::to_string(&SavedLyraProfile::capture(settings)).map_err(|_| {
                DockError(gettext(
                    "Não foi possível alterar o perfil da área de trabalho.",
                ))
            })?,
        )
    } else {
        None
    };
    settings.delay();
    let result = (|| {
        if let Some(snapshot) = snapshot {
            settings.set_string("lyra-profile-settings", &snapshot)?;
        }
        if profile == DesktopProfile::Ubuntu {
            settings.set_string("position", "left")?;
            settings.set_boolean("extend-to-edges", true)?;
            settings.set_boolean("floating-panel", true)?;
            settings.set_string("extended-content-alignment", "start")?;
            for key in PROFILE_MENUS {
                settings.set_boolean(key, false)?;
            }
            settings.set_string("desktop-profile", "ubuntu")?;
        } else {
            if let Some(saved) = saved {
                saved.restore(settings)?;
            }
            settings.set_string("desktop-profile", "lyra")?;
        }
        Ok::<_, glib::BoolError>(())
    })();
    if result.is_err() {
        settings.revert();
        return Err(DockError(gettext(
            "Não foi possível alterar o perfil da área de trabalho.",
        )));
    }
    settings.apply();
    Ok(())
}

fn alignment_key(settings: &gio::Settings, extended: bool) -> &'static str {
    if extended && has_key(settings, "extended-content-alignment") {
        "extended-content-alignment"
    } else {
        "content-alignment"
    }
}

pub fn is_installed() -> bool {
    extension_dir().is_some()
}

fn shell_settings() -> Option<gio::Settings> {
    gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SHELL_SCHEMA_ID, true))
        .map(|_| gio::Settings::new(SHELL_SCHEMA_ID))
}

pub fn is_enabled() -> bool {
    shell_settings().is_some_and(|settings| {
        settings
            .strv("enabled-extensions")
            .iter()
            .any(|uuid| uuid.as_str() == EXTENSION_UUID)
    })
}

pub fn set_enabled(enabled: bool) -> Result<(), DockError> {
    if enabled && !is_installed() {
        return Err(DockError(gettext(
            "A extensão Sheliak não está instalada ou não pôde ser encontrada.",
        )));
    }
    let settings = shell_settings().ok_or_else(|| {
        DockError(gettext(
            "As configurações de extensões do GNOME não estão disponíveis.",
        ))
    })?;
    let mut extensions = settings
        .strv("enabled-extensions")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    extensions.retain(|uuid| uuid != EXTENSION_UUID);
    if enabled {
        extensions.push(EXTENSION_UUID.to_string());
    }
    let extension_refs = extensions.iter().map(String::as_str).collect::<Vec<_>>();
    settings
        .set_strv("enabled-extensions", extension_refs)
        .map_err(|_| {
            DockError(gettext(
                "Não foi possível alterar o perfil da área de trabalho.",
            ))
        })
}

fn open_settings() -> Option<gio::Settings> {
    let dir = extension_dir()?;
    let source = gio::SettingsSchemaSource::from_directory(
        dir.join("schemas"),
        gio::SettingsSchemaSource::default().as_ref(),
        false,
    )
    .ok()?;
    let schema = source.lookup(SCHEMA_ID, false)?;
    Some(gio::Settings::new_full(
        &schema,
        None::<&gio::SettingsBackend>,
        Some(SCHEMA_PATH),
    ))
}

fn has_key(settings: &gio::Settings, key: &str) -> bool {
    settings
        .settings_schema()
        .is_some_and(|schema| schema.has_key(key))
}

fn string_or(settings: &gio::Settings, key: &str, fallback: &str) -> String {
    if has_key(settings, key) {
        settings.string(key).to_string()
    } else {
        fallback.to_string()
    }
}

fn boolean_or(settings: &gio::Settings, key: &str, fallback: bool) -> bool {
    if has_key(settings, key) {
        settings.boolean(key)
    } else {
        fallback
    }
}

fn uint_or(settings: &gio::Settings, key: &str, fallback: u32) -> u32 {
    if has_key(settings, key) {
        settings.uint(key)
    } else {
        fallback
    }
}

fn set_string_if_present(settings: &gio::Settings, key: &str, value: &str) {
    if has_key(settings, key) {
        let _ = settings.set_string(key, value);
    }
}

fn set_boolean_if_present(settings: &gio::Settings, key: &str, value: bool) {
    if has_key(settings, key) {
        let _ = settings.set_boolean(key, value);
    }
}

fn set_uint_if_present(settings: &gio::Settings, key: &str, value: u32) {
    if has_key(settings, key) {
        let _ = settings.set_uint(key, value);
    }
}

pub fn alignment_for_mode(extended: bool) -> Option<String> {
    let settings = open_settings()?;
    Some(
        settings
            .string(alignment_key(&settings, extended))
            .to_string(),
    )
}

pub fn current() -> Option<DockSettings> {
    let settings = open_settings()?;
    Some(DockSettings {
        position: settings.string("position").to_string(),
        hide_mode: settings.string("hide-mode").to_string(),
        hide_delay_ms: settings.uint("hide-delay"),
        icon_size: settings.uint("icon-size"),
        edge_margin: settings.uint("edge-margin"),
        animation: settings.boolean("animation"),
        minimize_animation: string_or(&settings, "minimize-animation", "zoom"),
        extend_to_edges: settings.boolean("extend-to-edges"),
        content_alignment: settings
            .string(alignment_key(
                &settings,
                settings.boolean("extend-to-edges"),
            ))
            .to_string(),
        show_running: settings.boolean("show-running"),
        running_apps_position: settings.string("running-apps-position").to_string(),
        show_trash: settings.boolean("show-trash"),
        show_apps_button: settings.boolean("show-apps-button"),
        fullscreen_hide: settings.boolean("fullscreen-hide"),
    })
}

pub fn current_menu() -> Option<MenuSettings> {
    let settings = open_settings()?;
    Some(MenuSettings {
        panel_height: uint_or(&settings, "panel-height", 32),
        floating_panel: boolean_or(&settings, "floating-panel", true),
        panel_margin: uint_or(&settings, "panel-margin", 8),
        show_clock: boolean_or(&settings, "show-clock", true),
        show_panel_indicators: boolean_or(&settings, "show-panel-indicators", true),
        show_applications_menu: boolean_or(&settings, "show-applications-menu", true),
        show_places_menu: boolean_or(&settings, "show-places-menu", true),
        show_network_menu: boolean_or(&settings, "show-network-menu", true),
        show_system_menu: boolean_or(&settings, "show-system-menu", true),
        show_system_about: boolean_or(&settings, "show-system-about", true),
        show_search_menu: boolean_or(&settings, "show-search-menu", true),
        hide_workspace_button: boolean_or(&settings, "hide-workspace-button", true),
        panel_menu_position: string_or(&settings, "panel-menu-position", "left"),
        show_application_icons: boolean_or(&settings, "show-application-icons", true),
        sort_applications_menu: boolean_or(&settings, "sort-applications-menu", true),
        open_application_submenus_sideways: boolean_or(
            &settings,
            "open-application-submenus-sideways",
            true,
        ),
        show_place_bookmarks: boolean_or(&settings, "show-place-bookmarks", true),
        show_place_volumes: boolean_or(&settings, "show-place-volumes", true),
    })
}

pub fn apply(settings: &DockSettings) -> Result<(), DockError> {
    let gsettings = open_settings().ok_or_else(|| {
        DockError(gettext(
            "A extensão Sheliak não está instalada ou não pôde ser encontrada.",
        ))
    })?;
    let _ = gsettings.set_string("position", &settings.position);
    let _ = gsettings.set_string("hide-mode", &settings.hide_mode);
    let _ = gsettings.set_uint("hide-delay", settings.hide_delay_ms);
    let _ = gsettings.set_uint("icon-size", settings.icon_size);
    let _ = gsettings.set_uint("edge-margin", settings.edge_margin);
    let _ = gsettings.set_boolean("animation", settings.animation);
    set_string_if_present(
        &gsettings,
        "minimize-animation",
        &settings.minimize_animation,
    );
    let _ = gsettings.set_boolean("extend-to-edges", settings.extend_to_edges);
    let _ = gsettings.set_string(
        alignment_key(&gsettings, settings.extend_to_edges),
        &settings.content_alignment,
    );
    let _ = gsettings.set_boolean("show-running", settings.show_running);
    let _ = gsettings.set_string("running-apps-position", &settings.running_apps_position);
    let _ = gsettings.set_boolean("show-trash", settings.show_trash);
    let _ = gsettings.set_boolean("show-apps-button", settings.show_apps_button);
    let _ = gsettings.set_boolean("fullscreen-hide", settings.fullscreen_hide);
    Ok(())
}

pub fn apply_menu(settings: &MenuSettings) -> Result<(), DockError> {
    let gsettings = open_settings().ok_or_else(|| {
        DockError(gettext(
            "A extensão Sheliak não está instalada ou não pôde ser encontrada.",
        ))
    })?;
    set_uint_if_present(&gsettings, "panel-height", settings.panel_height);
    set_boolean_if_present(&gsettings, "floating-panel", settings.floating_panel);
    set_uint_if_present(&gsettings, "panel-margin", settings.panel_margin);
    set_boolean_if_present(&gsettings, "show-clock", settings.show_clock);
    set_boolean_if_present(
        &gsettings,
        "show-panel-indicators",
        settings.show_panel_indicators,
    );
    set_boolean_if_present(
        &gsettings,
        "show-applications-menu",
        settings.show_applications_menu,
    );
    set_boolean_if_present(&gsettings, "show-places-menu", settings.show_places_menu);
    set_boolean_if_present(&gsettings, "show-network-menu", settings.show_network_menu);
    set_boolean_if_present(&gsettings, "show-system-menu", settings.show_system_menu);
    set_boolean_if_present(&gsettings, "show-system-about", settings.show_system_about);
    set_boolean_if_present(&gsettings, "show-search-menu", settings.show_search_menu);
    set_boolean_if_present(
        &gsettings,
        "hide-workspace-button",
        settings.hide_workspace_button,
    );
    set_string_if_present(
        &gsettings,
        "panel-menu-position",
        &settings.panel_menu_position,
    );
    set_boolean_if_present(
        &gsettings,
        "show-application-icons",
        settings.show_application_icons,
    );
    set_boolean_if_present(
        &gsettings,
        "sort-applications-menu",
        settings.sort_applications_menu,
    );
    set_boolean_if_present(
        &gsettings,
        "open-application-submenus-sideways",
        settings.open_application_submenus_sideways,
    );
    set_boolean_if_present(
        &gsettings,
        "show-place-bookmarks",
        settings.show_place_bookmarks,
    );
    set_boolean_if_present(
        &gsettings,
        "show-place-volumes",
        settings.show_place_volumes,
    );
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    fn settings() -> gio::Settings {
        settings_with_cache(false)
    }

    fn settings_with_cache(cache: bool) -> gio::Settings {
        let path = std::env::temp_dir().join(format!(
            "vega-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let mut xml = format!("<schemalist><schema id='{SCHEMA_ID}' path='{SCHEMA_PATH}'>");
        if cache {
            xml.push_str(
                "<key name='desktop-profile-settings' type='s'><default>''</default></key>",
            );
        }
        xml.push_str("<key name='icon-size' type='u'><default>40</default></key>");
        for (key, default) in [
            ("position", "right"),
            ("content-alignment", "center"),
            ("extended-content-alignment", "end"),
            ("desktop-profile", "lyra"),
            ("lyra-profile-settings", ""),
            ("hide-mode", "intelligent"),
        ] {
            xml.push_str(&format!(
                "<key name='{key}' type='s'><default>'{default}'</default></key>"
            ));
        }
        for key in [
            "extend-to-edges",
            "floating-panel",
            "show-applications-menu",
            "show-places-menu",
            "show-system-menu",
            "show-search-menu",
            "show-trash",
            "show-apps-button",
            "show-clock",
            "show-panel-indicators",
            "fullscreen-hide",
        ] {
            xml.push_str(&format!(
                "<key name='{key}' type='b'><default>true</default></key>"
            ));
        }
        xml.push_str("</schema></schemalist>");
        std::fs::write(path.join("test.gschema.xml"), xml).unwrap();
        assert!(
            std::process::Command::new("glib-compile-schemas")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let source = gio::SettingsSchemaSource::from_directory(&path, None, false).unwrap();
        let schema = source.lookup(SCHEMA_ID, false).unwrap();
        let settings = gio::Settings::new_full(
            &schema,
            Some(&gio::memory_settings_backend_new()),
            Some(SCHEMA_PATH),
        );
        std::fs::remove_dir_all(path).unwrap();
        settings
    }

    #[test]
    fn only_explicit_ubuntu_hides_menus_and_lyra_restores_custom_preferences() {
        let settings = settings();
        settings.set_boolean("show-places-menu", false).unwrap();
        let original = SavedLyraProfile::capture(&settings);
        // A full-length dock is still Lyra and must keep its menus/search.
        assert!(settings.boolean("extend-to-edges"));
        assert_eq!(settings.string("desktop-profile"), "lyra");
        assert!(settings.boolean("show-search-menu"));
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        assert_eq!(settings.string("desktop-profile"), "ubuntu");
        assert_eq!(settings.string("position"), "left");
        assert_eq!(settings.string("extended-content-alignment"), "start");
        assert_eq!(settings.string("content-alignment"), "center");
        assert!(PROFILE_MENUS.iter().all(|key| !settings.boolean(key)));
        // Re-selecting Ubuntu must not replace the saved Lyra profile.
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(settings.string("desktop-profile"), "lyra");
        assert_eq!(SavedLyraProfile::capture(&settings), original);
    }

    #[test]
    fn corrupt_saved_profile_is_reported_without_partial_changes() {
        let settings = settings();
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        settings
            .set_string("lyra-profile-settings", "broken")
            .unwrap();
        settings.apply();
        let before = SavedLyraProfile::capture(&settings);
        assert!(apply_profile_settings(&settings, DesktopProfile::Lyra).is_err());
        assert_eq!(settings.string("desktop-profile"), "ubuntu");
        assert_eq!(SavedLyraProfile::capture(&settings), before);
    }

    #[test]
    fn windows_profiles_restore_all_previous_layout_and_visibility_preferences() {
        let settings = settings_with_cache(true);
        settings.set_boolean("show-clock", false).unwrap();
        settings.set_boolean("show-places-menu", false).unwrap();
        let lyra = SavedDesktopProfile::capture(&settings);
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        settings.set_uint("icon-size", 44).unwrap();
        settings.apply();
        let ubuntu = SavedDesktopProfile::capture(&settings);
        apply_profile_settings(&settings, DesktopProfile::Windows10).unwrap();
        assert_eq!(settings.string("position"), "bottom");
        assert_eq!(settings.string("hide-mode"), "always");
        assert_eq!(settings.string("extended-content-alignment"), "start");
        assert!(settings.boolean("show-clock"));
        assert!(!settings.boolean("show-trash"));
        assert_eq!(settings.uint("icon-size"), 32);
        apply_profile_settings(&settings, DesktopProfile::Windows11).unwrap();
        assert_eq!(settings.string("extended-content-alignment"), "center");
        assert_eq!(settings.uint("icon-size"), 28);
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), ubuntu);
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);
        apply_profile_settings(&settings, DesktopProfile::Windows10).unwrap();
        assert_eq!(settings.string("desktop-profile"), "windows10");
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);
    }

    #[test]
    fn windows_migrate_existing_ubuntu_backup_and_reject_corrupt_cache() {
        let settings = settings_with_cache(true);
        let lyra = SavedDesktopProfile::capture(&settings);
        settings
            .set_string(
                "lyra-profile-settings",
                &serde_json::to_string(&lyra.layout).unwrap(),
            )
            .unwrap();
        settings.set_string("desktop-profile", "ubuntu").unwrap();
        for key in PROFILE_MENUS {
            settings.set_boolean(key, false).unwrap();
        }
        apply_profile_settings(&settings, DesktopProfile::Windows11).unwrap();
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);
        settings
            .set_string("desktop-profile-settings", "broken")
            .unwrap();
        settings.apply();
        assert!(apply_profile_settings(&settings, DesktopProfile::Windows10).is_err());
        assert_eq!(settings.string("desktop-profile"), "lyra");
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);
    }
}
