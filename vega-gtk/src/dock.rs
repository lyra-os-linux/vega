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
    extension_dir_for("dock@lyraos.com.br").or_else(|| extension_dir_for(EXTENSION_UUID))
}

fn extension_dir_for(uuid: &str) -> Option<PathBuf> {
    let mut candidates = vec![
        glib::user_data_dir()
            .join("gnome-shell/extensions")
            .join(uuid),
    ];
    for base in ["/usr/share", "/usr/local/share"] {
        candidates.push(
            PathBuf::from(base)
                .join("gnome-shell/extensions")
                .join(uuid),
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
    Macos,
    GnomeVanilla,
}

impl DesktopProfile {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "lyra" => Some(Self::Lyra),
            "vanilla" => Some(Self::GnomeVanilla),
            "ubuntu" => Some(Self::Ubuntu),
            "windows10" => Some(Self::Windows10),
            "windows11" => Some(Self::Windows11),
            "macos" => Some(Self::Macos),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Lyra => "lyra",
            Self::GnomeVanilla => "vanilla",
            Self::Ubuntu => "ubuntu",
            Self::Windows10 => "windows10",
            Self::Windows11 => "windows11",
            Self::Macos => "macos",
        }
    }
}

/// Strict read for setup clients: an unavailable/old backend must never be
/// mistaken for a successfully applied default. Values describe stored settings.
pub fn confirmed_profile() -> Result<DesktopProfile, DockError> {
    if suite_available() {
        let state = suite_command(&["status"])?;
        if state.globally_disabled && state.profile != "vanilla" {
            return Err(DockError("GNOME extensions are globally disabled".into()));
        }
        return DesktopProfile::from_id(&state.profile)
            .ok_or_else(|| DockError("Unknown suite profile".into()));
    }
    let unavailable = || DockError("desktop profile settings are unavailable".into());
    let shell = shell_settings().ok_or_else(unavailable)?;
    let settings = open_settings().ok_or_else(unavailable)?;
    if !has_key(&settings, "desktop-profile-settings") {
        return Err(unavailable());
    }
    let enabled = shell
        .strv("enabled-extensions")
        .iter()
        .any(|id| id == EXTENSION_UUID);
    if !enabled {
        return Ok(DesktopProfile::GnomeVanilla);
    }
    if shell.boolean("disable-user-extensions")
        || shell
            .strv("disabled-extensions")
            .iter()
            .any(|id| id == EXTENSION_UUID)
    {
        return Err(unavailable());
    }
    DesktopProfile::from_id(settings.string("desktop-profile").as_str())
        .filter(|profile| *profile != DesktopProfile::GnomeVanilla)
        .ok_or_else(unavailable)
}

pub fn current_profile() -> DesktopProfile {
    if suite_available() {
        return confirmed_profile().unwrap_or(DesktopProfile::GnomeVanilla);
    }
    if !is_enabled() {
        return DesktopProfile::GnomeVanilla;
    }
    open_settings().map_or(DesktopProfile::Lyra, |settings| {
        match string_or(&settings, "desktop-profile", "lyra").as_str() {
            "ubuntu" => DesktopProfile::Ubuntu,
            "windows10" => DesktopProfile::Windows10,
            "windows11" => DesktopProfile::Windows11,
            "macos" => DesktopProfile::Macos,
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
    // Missing in snapshots created before the workspace button was per-profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hide_workspace_button: Option<bool>,
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
    #[serde(default)]
    presentation: Option<SavedProfilePresentation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SavedProfilePresentation {
    alignment: String,
    animation: bool,
    margin: u32,
    menu_position: String,
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
            presentation: Some(SavedProfilePresentation {
                alignment: string_or(settings, "content-alignment", "center"),
                animation: boolean_or(settings, "animation", true),
                margin: uint_or(settings, "edge-margin", 8),
                menu_position: string_or(settings, "panel-menu-position", "left"),
            }),
        }
    }

    fn restore(&self, settings: &gio::Settings) -> Result<(), glib::BoolError> {
        self.layout.restore(settings)?;
        if let Some(extra) = &self.presentation {
            for (key, value) in [
                ("content-alignment", extra.alignment.as_str()),
                ("panel-menu-position", extra.menu_position.as_str()),
            ] {
                if has_key(settings, key) {
                    settings.set_string(key, value)?;
                }
            }
            if has_key(settings, "animation") {
                settings.set_boolean("animation", extra.animation)?;
            }
            if has_key(settings, "edge-margin") {
                settings.set_uint("edge-margin", extra.margin)?;
            }
        }
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
        DesktopProfile::Macos => "macos",
        DesktopProfile::GnomeVanilla => return Ok(()),
    };
    let current = settings.string("desktop-profile");
    if current == key && profile != DesktopProfile::Ubuntu {
        return Ok(());
    }
    let error = || {
        DockError(gettext(
            "Não foi possível restaurar as preferências dos perfis.",
        ))
    };
    if !["lyra", "ubuntu", "windows10", "windows11", "macos"].contains(&current.as_str()) {
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
        .any(|name| !["lyra", "ubuntu", "windows10", "windows11", "macos"].contains(&name.as_str()))
    {
        return Err(error());
    }
    let before = SavedDesktopProfile::capture(settings);
    let workspace_preference_saved = saved
        .get(current.as_str())
        .is_some_and(|value| value.layout.hide_workspace_button.is_some());
    // Re-selecting an existing Ubuntu profile applies its new default once.
    // Once migrated, re-selection must preserve explicit user customization.
    if current == key
        && (before.layout.hide_workspace_button.is_none() || workspace_preference_saved)
    {
        return Ok(());
    }
    // Older snapshots shared these settings globally. Seed them once from
    // that shared state before Lyra Flutuante changes them, preserving the migration.
    for value in saved.values_mut() {
        if value.presentation.is_none() {
            value.presentation = before.presentation.clone();
        }
    }
    // Migrate the Lyra snapshot created by the Ubuntu-only Vega version.
    if current == "ubuntu" && !saved.contains_key("lyra") {
        let layout: SavedLyraProfile =
            serde_json::from_str(&settings.string("lyra-profile-settings")).map_err(|_| error())?;
        let mut lyra = before.clone();
        lyra.layout = layout;
        saved.insert("lyra".into(), lyra);
    }
    // The old format shared this preference across profiles. Preserve that
    // value for every other profile and make Ubuntu's first value visible.
    for (name, value) in &mut saved {
        if value.layout.hide_workspace_button.is_none() {
            value.layout.hide_workspace_button = before
                .layout
                .hide_workspace_button
                .map(|hidden| name != "ubuntu" && hidden);
        }
    }
    let mut current_snapshot = before.clone();
    if current == "ubuntu" && !workspace_preference_saved {
        current_snapshot.layout.hide_workspace_button =
            before.layout.hide_workspace_button.map(|_| false);
    }
    saved.insert(current.to_string(), current_snapshot);
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
        if profile == DesktopProfile::Ubuntu {
            preset.layout.hide_workspace_button =
                before.layout.hide_workspace_button.map(|_| false);
        }
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
        if profile == DesktopProfile::Macos {
            preset.layout.position = "bottom".into();
            preset.layout.extended = false;
            preset.layout.floating = false;
            preset.layout.menus = [true, false, false, false];
            preset.hide_mode = "always".into();
            preset.icon_size = 48;
            preset.show_trash = true;
            preset.show_apps = true;
            preset.show_clock = true;
            preset.show_indicators = true;
            preset.fullscreen_hide = true;
            preset.presentation = Some(SavedProfilePresentation {
                alignment: "center".into(),
                animation: true,
                margin: 10,
                menu_position: "left".into(),
            });
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
            hide_workspace_button: has_key(settings, "hide-workspace-button")
                .then(|| settings.boolean("hide-workspace-button")),
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
        if let Some(hidden) = self.hide_workspace_button
            && has_key(settings, "hide-workspace-button")
        {
            settings.set_boolean("hide-workspace-button", hidden)?;
        }
        Ok(())
    }
}

pub fn supports_macos_profile() -> bool {
    open_settings().is_some_and(|s| has_key(&s, "macos-profile-supported"))
}

pub fn apply_profile(profile: DesktopProfile) -> Result<(), DockError> {
    if suite_available() {
        suite_command(&["begin-profile", profile.id()])?;
        let result = (|| {
            let settings =
                open_settings().ok_or_else(|| DockError("Suite settings unavailable".into()))?;
            if profile != DesktopProfile::GnomeVanilla {
                apply_profile_settings(&settings, profile)?;
            }
            suite_command(&["commit-profile", profile.id()])?;
            Ok(())
        })();
        if let Err(error) = result {
            if let Err(recovery) = suite_command(&["abort-profile", profile.id()]) {
                return Err(DockError(format!("{error}; {recovery}")));
            }
            return Err(error);
        }
        return Ok(());
    }
    if profile == DesktopProfile::Macos && !supports_macos_profile() {
        return Err(DockError(gettext(
            "Atualize o Sheliak para usar o perfil Lyra Flutuante.",
        )));
    }
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
    let mut saved = if was_ubuntu {
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
    if let Some(saved) = &mut saved
        && saved.hide_workspace_button.is_none()
        && has_key(settings, "hide-workspace-button")
    {
        saved.hide_workspace_button = Some(settings.boolean("hide-workspace-button"));
    }
    let snapshot = if profile == DesktopProfile::Ubuntu {
        Some(
            serde_json::to_string(
                &saved
                    .clone()
                    .unwrap_or_else(|| SavedLyraProfile::capture(settings)),
            )
            .map_err(|_| {
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
            if has_key(settings, "hide-workspace-button") {
                settings.set_boolean("hide-workspace-button", false)?;
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

const DESKTOP_ICONS_UUID: &str = "ding@rastersoft.com";

pub fn desktop_icons_state() -> Option<bool> {
    if suite_available() {
        return suite_components()
            .map(|items| items.get("desktop-icons").copied().unwrap_or(false));
    }
    extension_dir_for(DESKTOP_ICONS_UUID)?;
    let shell = shell_settings()?;
    Some(
        !shell.boolean("disable-user-extensions")
            && shell
                .strv("enabled-extensions")
                .iter()
                .any(|id| id == DESKTOP_ICONS_UUID)
            && !shell
                .strv("disabled-extensions")
                .iter()
                .any(|id| id == DESKTOP_ICONS_UUID),
    )
}

pub fn set_desktop_icons_enabled(enabled: bool) -> Result<(), DockError> {
    if suite_available() {
        return set_suite_component("desktop-icons", enabled);
    }
    let error = || {
        DockError(gettext(
            "Não foi possível alterar a área de trabalho ativa.",
        ))
    };
    if extension_dir_for(DESKTOP_ICONS_UUID).is_none() {
        return Err(error());
    }
    let settings = shell_settings().ok_or_else(error)?;
    if enabled && settings.boolean("disable-user-extensions") {
        return Err(error());
    }
    update_desktop_icons_setting(&settings, enabled).map_err(|_| error())?;
    gio::Settings::sync();
    if desktop_icons_state() != Some(enabled) {
        return Err(error());
    }
    Ok(())
}

fn update_desktop_icons_setting(
    settings: &gio::Settings,
    enabled: bool,
) -> Result<(), glib::BoolError> {
    let before = settings.strv("enabled-extensions");
    let mut active = before
        .iter()
        .filter(|id| id.as_str() != DESKTOP_ICONS_UUID)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if enabled {
        active.push(DESKTOP_ICONS_UUID.into());
    }
    let disabled = settings.strv("disabled-extensions");
    let allowed = disabled
        .iter()
        .filter(|id| id.as_str() != DESKTOP_ICONS_UUID)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    settings.delay();
    let result = (|| {
        if before.iter().map(|s| s.as_str()).collect::<Vec<_>>()
            != active.iter().map(String::as_str).collect::<Vec<_>>()
        {
            settings.set_strv(
                "enabled-extensions",
                active.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
        }
        if enabled && allowed.len() != disabled.len() {
            settings.set_strv(
                "disabled-extensions",
                allowed.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
        }
        Ok(())
    })();
    if result.is_err() {
        settings.revert();
    } else {
        settings.apply();
    }
    result
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
    if suite_available() {
        return suite_command(&["status"]).is_ok_and(|state| {
            !state.globally_disabled
                && state
                    .components
                    .iter()
                    .any(|(key, enabled)| key != "desktop-icons" && *enabled)
        });
    }
    shell_settings().is_some_and(|settings| {
        settings
            .strv("enabled-extensions")
            .iter()
            .any(|uuid| uuid.as_str() == EXTENSION_UUID)
    })
}

pub fn set_enabled(enabled: bool) -> Result<(), DockError> {
    if suite_available() {
        let profile = if enabled {
            open_settings()
                .map(|s| s.string("desktop-profile").to_string())
                .unwrap_or("lyra".into())
        } else {
            "vanilla".into()
        };
        return suite_command(&["apply", &profile]).map(|_| ());
    }
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

#[derive(serde::Deserialize)]
struct SuiteState {
    version: u32,
    profile: String,
    globally_disabled: bool,
    components: std::collections::BTreeMap<String, bool>,
}

pub fn suite_available() -> bool {
    extension_dir_for("dock@lyraos.com.br").is_some()
}

fn suite_command(args: &[&str]) -> Result<SuiteState, DockError> {
    let mut helper = PathBuf::from("/usr/libexec/lyra/shell-suite");
    // The private compositor fixture can exercise the real helper without installing it.
    if std::env::var("SHELIAK_PRIVATE_NATIVE_TEST").as_deref() == Ok("1")
        && std::env::var("HOME").is_ok_and(|home| home.starts_with("/tmp/sheliak-pins-"))
        && let Some(path) = std::env::var_os("LYRA_NATIVE_SUITE_HELPER")
    {
        helper = PathBuf::from(path);
    }
    let output = std::process::Command::new(helper)
        .args(args)
        .output()
        .map_err(|error| {
            DockError(format!(
                "{}: {error}",
                gettext("Não foi possível alterar o perfil da área de trabalho.")
            ))
        })?;
    if !output.status.success() {
        return Err(DockError(format!(
            "{}: {}",
            gettext("Não foi possível alterar o perfil da área de trabalho."),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let state: SuiteState = serde_json::from_slice(&output.stdout)
        .map_err(|_| DockError("Invalid Lyra suite response".into()))?;
    if state.version != 1
        || DesktopProfile::from_id(&state.profile).is_none()
        || state
            .components
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != [
                "animations",
                "desktop-icons",
                "dock",
                "menus",
                "panel",
                "search",
            ]
    {
        return Err(DockError("Unsupported Lyra suite response".into()));
    }
    Ok(state)
}

/// Context for editing preferences. A failed status query disables dependent controls.
#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsContext {
    pub dock: bool,
    pub panel: bool,
    pub menus: bool,
    pub search: bool,
    pub animations: bool,
    pub fixed_panel: bool,
    pub fixed_menu_position: bool,
    pub extended_dock: bool,
}

pub fn settings_context() -> SettingsContext {
    let profile = current_profile();
    let active = |role: &str, state: &Option<std::collections::BTreeMap<String, bool>>| {
        state
            .as_ref()
            .and_then(|s| s.get(role))
            .copied()
            .unwrap_or(false)
    };
    let state = if suite_available() {
        suite_components()
    } else {
        let enabled = is_installed() && is_enabled() && profile != DesktopProfile::GnomeVanilla;
        Some(
            ["dock", "panel", "menus", "search", "animations"]
                .into_iter()
                .map(|role| (role.to_string(), enabled))
                .collect(),
        )
    };
    SettingsContext {
        dock: active("dock", &state),
        panel: active("panel", &state),
        menus: active("menus", &state),
        search: active("search", &state),
        animations: active("animations", &state),
        fixed_panel: matches!(
            profile,
            DesktopProfile::Windows10 | DesktopProfile::Windows11
        ),
        fixed_menu_position: profile == DesktopProfile::Macos,
        extended_dock: current().is_none_or(|settings| settings.extend_to_edges),
    }
}

pub fn suite_components() -> Option<std::collections::BTreeMap<String, bool>> {
    if !suite_available() {
        return None;
    }
    suite_command(&["status"]).ok().map(|state| {
        state
            .components
            .into_iter()
            .map(|(role, active)| (role, active && !state.globally_disabled))
            .collect()
    })
}

pub fn set_suite_component(role: &str, active: bool) -> Result<(), DockError> {
    let state = suite_command(&["toggle", role, if active { "on" } else { "off" }])?;
    if state.components.get(role).copied() != Some(active) || active && state.globally_disabled {
        return Err(DockError(
            "Component preference could not be confirmed".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn desktop_toggle_preserves_other_extensions_and_global_preference() {
        // This unit test must not depend on GNOME Shell being installed on the builder.
        let shell = settings_from_xml(
            "org.gnome.shell",
            "/org/gnome/shell/",
            r#"<schemalist><schema id="org.gnome.shell" path="/org/gnome/shell/">
                <key name="enabled-extensions" type="as"><default>[]</default></key>
                <key name="disabled-extensions" type="as"><default>[]</default></key>
                <key name="disable-user-extensions" type="b"><default>false</default></key>
            </schema></schemalist>"#,
        );
        shell
            .set_strv(
                "enabled-extensions",
                [EXTENSION_UUID, DESKTOP_ICONS_UUID, "other@example.org"],
            )
            .unwrap();
        shell
            .set_strv(
                "disabled-extensions",
                ["blocked@example.org", DESKTOP_ICONS_UUID],
            )
            .unwrap();
        shell.set_boolean("disable-user-extensions", true).unwrap();
        update_desktop_icons_setting(&shell, false).unwrap();
        assert_eq!(
            shell.strv("enabled-extensions").as_slice(),
            [EXTENSION_UUID, "other@example.org"]
        );
        assert_eq!(
            shell.strv("disabled-extensions").as_slice(),
            ["blocked@example.org", DESKTOP_ICONS_UUID]
        );
        update_desktop_icons_setting(&shell, true).unwrap();
        update_desktop_icons_setting(&shell, true).unwrap();
        assert_eq!(
            shell.strv("enabled-extensions").as_slice(),
            [EXTENSION_UUID, "other@example.org", DESKTOP_ICONS_UUID]
        );
        assert_eq!(
            shell.strv("disabled-extensions").as_slice(),
            ["blocked@example.org"]
        );
        assert!(shell.boolean("disable-user-extensions"));
    }

    #[test]
    #[ignore = "requires disposable GNOME compositor and packaged DING"]
    fn desktop_icons_native_toggle() {
        assert_eq!(
            std::env::var("DING_PRIVATE_NATIVE_TEST").as_deref(),
            Ok("1")
        );
        let home = std::env::var("HOME").unwrap();
        assert!(home.starts_with("/tmp/sheliak-pins-"));
        let fixture = std::path::Path::new(&home).join("Desktop/Fixture.txt");
        let before = std::fs::read(&fixture).unwrap();
        let enabled = match std::env::var("VEGA_DESKTOP_ICONS_TEST_ACTION").as_deref() {
            Ok("enable") => true,
            Ok("disable") => false,
            _ => panic!("explicit action required"),
        };
        let shell = shell_settings().unwrap();
        let others = shell
            .strv("enabled-extensions")
            .iter()
            .filter(|id| id.as_str() != DESKTOP_ICONS_UUID)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        set_desktop_icons_enabled(enabled).unwrap();
        assert_eq!(desktop_icons_state(), Some(enabled));
        assert_eq!(
            shell
                .strv("enabled-extensions")
                .iter()
                .filter(|id| id.as_str() != DESKTOP_ICONS_UUID)
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            others
        );
        for profile in [
            DesktopProfile::Macos,
            DesktopProfile::Windows10,
            DesktopProfile::Windows11,
            DesktopProfile::Ubuntu,
            DesktopProfile::GnomeVanilla,
            DesktopProfile::Lyra,
        ] {
            apply_profile(profile).unwrap();
            assert_eq!(desktop_icons_state(), Some(enabled));
        }
        assert_eq!(std::fs::read(fixture).unwrap(), before);
    }

    fn settings() -> gio::Settings {
        settings_with_cache(false)
    }

    fn settings_with_cache(cache: bool) -> gio::Settings {
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
            "hide-workspace-button",
        ] {
            xml.push_str(&format!(
                "<key name='{key}' type='b'><default>true</default></key>"
            ));
        }
        xml.push_str("</schema></schemalist>");
        settings_from_xml(SCHEMA_ID, SCHEMA_PATH, &xml)
    }

    fn settings_from_xml(id: &str, schema_path: &str, xml: &str) -> gio::Settings {
        let path = std::env::temp_dir().join(format!(
            "vega-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("test.gschema.xml"), xml).unwrap();
        assert!(
            std::process::Command::new("glib-compile-schemas")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let source = gio::SettingsSchemaSource::from_directory(&path, None, false).unwrap();
        let schema = source.lookup(id, false).unwrap();
        let settings = gio::Settings::new_full(
            &schema,
            Some(&gio::memory_settings_backend_new()),
            Some(schema_path),
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
        assert!(!settings.boolean("hide-workspace-button"));
        // Re-selecting Ubuntu must not replace the saved Lyra profile.
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(settings.string("desktop-profile"), "lyra");
        assert_eq!(SavedLyraProfile::capture(&settings), original);
        assert!(settings.boolean("hide-workspace-button"));
    }

    #[test]
    fn ubuntu_workspace_button_is_visible_and_other_profiles_keep_their_preferences() {
        let settings = settings_with_cache(true);
        let lyra = SavedDesktopProfile::capture(&settings);
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        assert!(!settings.boolean("hide-workspace-button"));
        assert!(PROFILE_MENUS.iter().all(|key| !settings.boolean(key)));
        for profile in [
            DesktopProfile::Windows10,
            DesktopProfile::Windows11,
            DesktopProfile::Macos,
        ] {
            apply_profile_settings(&settings, profile).unwrap();
            assert!(settings.boolean("hide-workspace-button"));
            apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
            assert!(!settings.boolean("hide-workspace-button"));
        }
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);

        // Explicit customization remains a per-profile choice after migration.
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        settings.set_boolean("hide-workspace-button", true).unwrap();
        settings.apply();
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        assert!(settings.boolean("hide-workspace-button"));
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        settings
            .set_boolean("hide-workspace-button", false)
            .unwrap();
        settings.apply();
        apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
        assert!(settings.boolean("hide-workspace-button"));
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert!(!settings.boolean("hide-workspace-button"));
    }

    #[test]
    fn ubuntu_workspace_button_migrates_old_cache_without_resetting_layouts() {
        for current in [DesktopProfile::Lyra, DesktopProfile::Ubuntu] {
            let settings = settings_with_cache(true);
            apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
            settings.set_uint("icon-size", 44).unwrap();
            apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
            apply_profile_settings(&settings, current).unwrap();
            let mut old: serde_json::Value =
                serde_json::from_str(&settings.string("desktop-profile-settings")).unwrap();
            for value in old.as_object_mut().unwrap().values_mut() {
                value["layout"]
                    .as_object_mut()
                    .unwrap()
                    .remove("hide_workspace_button");
            }
            settings
                .set_string("desktop-profile-settings", &old.to_string())
                .unwrap();
            settings.set_boolean("hide-workspace-button", true).unwrap();
            settings.apply();

            apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
            assert!(!settings.boolean("hide-workspace-button"));
            assert_eq!(settings.uint("icon-size"), 44);
            assert!(PROFILE_MENUS.iter().all(|key| !settings.boolean(key)));
            apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
            assert!(settings.boolean("hide-workspace-button"));
            assert_eq!(settings.string("position"), "right");
            apply_profile_settings(&settings, DesktopProfile::Ubuntu).unwrap();
            assert!(!settings.boolean("hide-workspace-button"));
        }
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
    fn macos_profile_migrates_old_snapshots_and_restores_other_layouts() {
        let settings = settings_with_cache(true);
        let lyra = SavedDesktopProfile::capture(&settings);
        apply_profile_settings(&settings, DesktopProfile::Windows10).unwrap();
        let windows = SavedDesktopProfile::capture(&settings);
        // Simulate the snapshot format shipped before Lyra Flutuante.
        let mut old: serde_json::Value =
            serde_json::from_str(&settings.string("desktop-profile-settings")).unwrap();
        for value in old.as_object_mut().unwrap().values_mut() {
            value.as_object_mut().unwrap().remove("presentation");
        }
        settings
            .set_string(
                "desktop-profile-settings",
                &serde_json::to_string(&old).unwrap(),
            )
            .unwrap();
        settings.apply();
        apply_profile_settings(&settings, DesktopProfile::Macos).unwrap();
        assert_eq!(settings.string("position"), "bottom");
        assert!(!settings.boolean("extend-to-edges"));
        assert!(!settings.boolean("floating-panel"));
        assert!(settings.boolean("show-applications-menu"));
        assert!(!settings.boolean("show-search-menu"));
        settings.set_uint("icon-size", 52).unwrap();
        settings.apply();
        apply_profile_settings(&settings, DesktopProfile::Windows10).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), windows);
        apply_profile_settings(&settings, DesktopProfile::Lyra).unwrap();
        assert_eq!(SavedDesktopProfile::capture(&settings), lyra);
        apply_profile_settings(&settings, DesktopProfile::Macos).unwrap();
        assert_eq!(settings.uint("icon-size"), 52);
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
