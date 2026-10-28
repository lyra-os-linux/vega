//! Comandos do próprio aplicativo: versão, página inicial, catálogo de
//! tradução e preferências.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

use crate::{assistant, i18n, model, preferences};

pub struct Startup {
    pub started: Instant,
}

/// Página pedida na linha de comando: `--virtual-machines` (atalho das VMs)
/// ou `--open-updates` (notificador de atualizações).
pub fn requested_page(argv: &[String]) -> Option<&'static str> {
    if argv.iter().any(|arg| arg == "--virtual-machines") {
        Some("virtualization")
    } else if argv.iter().any(|arg| arg == "--open-updates") {
        Some("software/updates")
    } else {
        None
    }
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    locale: &'static str,
    /// Página da primeira janela: pedido explícito ou a preferência.
    start: String,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    let argv: Vec<String> = std::env::args().collect();
    let start = requested_page(&argv)
        .map(str::to_owned)
        .or_else(|| {
            (std::env::var_os("VEGA_START_PAGE").as_deref()
                == Some(std::ffi::OsStr::new("software")))
            .then(|| "software/updates".to_owned())
        })
        .or_else(|| {
            // Só em builds de desenvolvimento: abrir direto numa página.
            cfg!(debug_assertions)
                .then(|| std::env::var("VEGA_DEV_PAGE").ok())
                .flatten()
        })
        .unwrap_or_else(|| preferences::load().start_page);
    AppInfo {
        version: model::APPLICATION_VERSION.to_owned(),
        locale: i18n::active_locale(),
        start,
    }
}

/// A interface avisa quando a primeira página foi desenhada.
#[tauri::command]
pub fn app_ready(startup: tauri::State<'_, Startup>) {
    if std::env::var_os("VEGA_BENCHMARK_MARKER").is_some() {
        eprintln!(
            "VEGA_WINDOW_READY_MS={}",
            startup.started.elapsed().as_millis()
        );
    }
}

#[tauri::command]
pub fn i18n_catalog() -> HashMap<String, String> {
    i18n::catalog()
}

#[tauri::command]
pub fn prefs_get() -> preferences::Settings {
    preferences::load()
}

#[tauri::command]
pub fn prefs_set(settings: preferences::Settings) -> Result<(), String> {
    let previous = preferences::load();
    preferences::save(&settings);
    if previous.save_ai_history && !settings.save_ai_history {
        assistant::clear_history().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Abre um endereço no aplicativo padrão (site, issues).
#[tauri::command]
pub fn open_uri(uri: String) -> Result<(), String> {
    if !(uri.starts_with("https://") || uri.starts_with("file://")) {
        return Err("endereço não permitido".into());
    }
    let context = gio::AppLaunchContext::new();
    crate::webkit_env::clean_launch_context(&context);
    gio::AppInfo::launch_default_for_uri(&uri, Some(&context)).map_err(|e| e.to_string())
}

/// Erros não tratados da interface vão para o stderr (journal da sessão).
#[tauri::command]
pub fn app_log(message: String) {
    eprintln!("vega-ui: {message}");
}
