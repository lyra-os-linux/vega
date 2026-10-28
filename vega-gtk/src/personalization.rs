//! Personalização: tema, perfis do desktop (Sheliak), dock, menu, tela de
//! bloqueio e atalhos para os apps do GNOME. Tudo roda na sessão do
//! usuário (gsettings, shell-suite), sem o vegad.
//!
//! A lógica fica em `dock`, `screensaver`, `appearance` e `tweaks` (a mesma
//! usada por `--desktop-profile`); aqui só há a ponte com a interface. As
//! operações rodam na thread principal, dona do contexto do GLib, como no
//! vega-gtk.

use std::sync::mpsc;

use gio::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use crate::appearance::{self, Theme};
use crate::dock::{self, DesktopProfile, DockSettings, MenuSettings};
use crate::i18n::gettext;
use crate::screensaver::{self, ScreensaverSettings};
use crate::tweaks;

const INTERFACE: &str = "org.gnome.desktop.interface";

const SETTINGS: &[&str] = &["org.gnome.Settings.desktop"];
const BACKGROUND: &[&str] = &["gnome-background-panel.desktop"];
const TWEAKS: &[&str] = &["org.gnome.tweaks.desktop", "gnome-tweak-tool.desktop"];
const EXTENSIONS: &[&str] = &[
    "org.gnome.Extensions.desktop",
    "org.gnome.Shell.Extensions.desktop",
];

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

fn interface_setting(key: &str) -> Option<String> {
    let settings = interface_schema()?;
    settings
        .settings_schema()
        .filter(|schema| schema.has_key(key))
        .map(|_| settings.string(key).to_string())
}

/// Mesmo filtro do vega-gtk: o GNOME Shell instala um .desktop com
/// `Exec=false` só para identificar o portal, mesmo sem o app Extensões.
fn find_app(ids: &[&str]) -> Option<gio::AppInfo> {
    let apps = gio::AppInfo::all();
    ids.iter().find_map(|id| {
        apps.iter()
            .find(|app| {
                app.id().as_deref() == Some(*id)
                    && app.executable().file_name() != Some(std::ffi::OsStr::new("false"))
            })
            .cloned()
    })
}

fn app_ids(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "settings" => Some(SETTINGS),
        "background" => Some(BACKGROUND),
        "tweaks" => Some(TWEAKS),
        "extensions" => Some(EXTENSIONS),
        _ => None,
    }
}

fn theme_id(theme: Theme) -> &'static str {
    match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
        Theme::System => "default",
    }
}

fn overview() -> Value {
    let app = |name: &str| {
        let found = app_ids(name).and_then(find_app);
        json!({
            "available": found.is_some(),
            "name": found.map(|app| app.display_name().to_string()),
        })
    };
    json!({
        "theme": appearance::schema_available().then(|| theme_id(appearance::current_theme())),
        "accent": interface_setting("accent-color"),
        "icon_theme": interface_setting("icon-theme"),
        "font_name": interface_setting("font-name"),
        "cursor_theme": interface_setting("cursor-theme"),
        "profile": dock::current_profile().id(),
        "sheliak": dock::is_installed() && dock::is_enabled(),
        "apps": {
            "settings": app("settings"),
            "background": app("background"),
            "tweaks": app("tweaks"),
            "extensions": app("extensions"),
        },
    })
}

fn profile_state() -> Value {
    let suite = dock::suite_available();
    json!({
        "installed": dock::is_installed(),
        "macos_supported": dock::supports_macos_profile(),
        "active": dock::confirmed_profile().ok().map(DesktopProfile::id),
        "suite": suite,
        "components": dock::suite_components(),
        "editable": dock::current_profile() != DesktopProfile::GnomeVanilla,
        "desktop_icons": dock::desktop_icons_state(),
    })
}

fn tweaks_page(name: &str) -> Option<tweaks::Page> {
    match name {
        "appearance" => Some(tweaks::Page::Appearance),
        "fonts" => Some(tweaks::Page::Fonts),
        "windows" => Some(tweaks::Page::Windows),
        _ => None,
    }
}

/// Abre um app do GNOME. Para escolher uma aba do Ajustes via AT-SPI, a
/// acessibilidade é ligada só nesse app.
fn launch(name: &str, accessibility: bool) -> Result<(), String> {
    let ids = app_ids(name).ok_or_else(|| format!("aplicativo desconhecido: {name}"))?;
    let app = find_app(ids).ok_or_else(|| gettext("Aplicativo não instalado"))?;
    let context = gio::AppLaunchContext::new();
    crate::webkit_env::clean_launch_context(&context);
    if accessibility {
        context.setenv("GTK_A11Y", "atspi");
    }
    app.launch(&[], Some(&context)).map_err(|error| {
        gettext("Não foi possível abrir {name}: {error}")
            .replace("{name}", &app.display_name())
            .replace("{error}", &error.to_string())
    })
}

fn arg<T: for<'de> Deserialize<'de>>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| format!("argumento inválido {key}: {error}"))
}

fn to_value(value: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

/// Executa `f` na thread principal e espera o resultado.
async fn on_main<F>(app: &AppHandle, f: F) -> Result<Value, String>
where
    F: FnOnce() -> Result<Value, String> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|_| "operação de personalização interrompida".to_string())?
}

/// Seleciona a aba do Ajustes no loop do GLib. `false`: um clique mais novo
/// substituiu este pedido, e o resultado deve ser ignorado.
async fn select_tweaks_page(app: &AppHandle, page: tweaks::Page) -> Result<Value, String> {
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        glib::MainContext::default().spawn_local(async move {
            let _ = tx.send(tweaks::select_page(page).await);
        });
    })
    .map_err(|error| error.to_string())?;
    let selected = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|_| "operação de personalização interrompida".to_string())??;
    Ok(Value::Bool(selected))
}

/// Entrada única dos comandos de personalização: `op` escolhe a operação.
#[tauri::command]
pub async fn personalization_call(
    app: AppHandle,
    op: String,
    args: Value,
) -> Result<Value, String> {
    if op == "open_app" {
        let name: String = arg(&args, "app")?;
        let page = arg::<Option<String>>(&args, "tweaks_page")?
            .map(|page| tweaks_page(&page).ok_or_else(|| format!("página desconhecida: {page}")))
            .transpose()?;
        let accessibility = page.is_some();
        on_main(&app, move || {
            launch(&name, accessibility).map(|_| Value::Null)
        })
        .await?;
        return match page {
            Some(page) => select_tweaks_page(&app, page).await,
            None => Ok(Value::Null),
        };
    }
    // O tema é lido já na abertura da janela, antes de o loop principal estar
    // livre; GSettings pode ser lido de qualquer thread.
    if op == "interface" {
        return dispatch(&op, &args);
    }
    on_main(&app, move || dispatch(&op, &args)).await
}

fn dispatch(op: &str, args: &Value) -> Result<Value, String> {
    match op {
        // Esquema de cor e destaque do GNOME, para a janela acompanhar o desktop.
        "interface" => Ok(interface_schema()
            .map(|settings| interface_state(&settings))
            .unwrap_or_else(|| json!({ "color_scheme": "default", "accent": "blue" }))),
        "overview" => Ok(overview()),
        "theme_set" => {
            if !appearance::schema_available() {
                return Err(gettext(
                    "Este sistema não tem os esquemas do GNOME para aparência; as opções abaixo ficam desativadas.",
                ));
            }
            // A interface nunca grava "default": só claro ou escuro.
            let dark: bool = arg(args, "dark")?;
            appearance::apply_theme(if dark { Theme::Dark } else { Theme::Light });
            Ok(Value::Null)
        }
        "profile_state" => Ok(profile_state()),
        "profile_apply" => {
            let id: String = arg(args, "id")?;
            let profile =
                DesktopProfile::from_id(&id).ok_or_else(|| format!("perfil desconhecido: {id}"))?;
            dock::apply_profile(profile).map_err(|error| error.to_string())?;
            Ok(profile_state())
        }
        "component_set" => {
            let role: String = arg(args, "role")?;
            let on: bool = arg(args, "on")?;
            let result = dock::set_suite_component(&role, on).map_err(|error| error.to_string());
            Ok(json!({ "error": result.err(), "state": profile_state() }))
        }
        "desktop_icons_set" => {
            let on: bool = arg(args, "on")?;
            let result = dock::set_desktop_icons_enabled(on).map_err(|error| error.to_string());
            Ok(json!({ "error": result.err(), "state": dock::desktop_icons_state() }))
        }
        "context" => to_value(dock::settings_context()),
        "dock_get" => Ok(json!({
            "settings": dock::current(),
            "context": dock::settings_context(),
        })),
        "dock_apply" => {
            let settings: DockSettings = arg(args, "settings")?;
            dock::apply(&settings).map_err(|error| error.to_string())?;
            to_value(dock::settings_context())
        }
        "dock_alignment" => {
            let extended: bool = arg(args, "extended")?;
            to_value(dock::alignment_for_mode(extended))
        }
        "menu_get" => Ok(json!({
            "settings": dock::current_menu(),
            "context": dock::settings_context(),
        })),
        "menu_apply" => {
            let settings: MenuSettings = arg(args, "settings")?;
            dock::apply_menu(&settings).map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
        "screensaver_get" => to_value(screensaver::current()),
        "screensaver_apply" => {
            let settings: ScreensaverSettings = arg(args, "settings")?;
            screensaver::apply(&settings).map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
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
    // GSettings só emite "changed" para chaves já lidas.
    let _ = settings.string("color-scheme");
    if settings
        .settings_schema()
        .is_some_and(|schema| schema.has_key("accent-color"))
    {
        let _ = settings.string("accent-color");
    }
    // A assinatura vive enquanto o processo viver.
    std::mem::forget(settings);
}
