//! Comandos do próprio aplicativo: versão, página inicial, catálogo de
//! tradução e preferências.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

use crate::{assistant, i18n, model, preferences};

pub struct Startup {
    pub started: Instant,
}

/// Esquema dos links que abrem o Vega numa página (vega#209), registrado no
/// vega.desktop: `vega://software/developers/databases`.
const LINK_SCHEME: &str = "vega://";

/// Destino pedido na linha de comando, como `página/aba/seção`:
/// `--virtual-machines` (atalho das VMs), `--open-updates` ou `--open-health`
/// (notificador), `--page P [--tab A [--section S]]` (Welcome, documentação)
/// ou um link `vega://P/A/S`. Destino malformado é ignorado e o Vega abre na
/// página inicial; a interface confere se página e seção existem.
pub fn requested_page(argv: &[String]) -> Option<String> {
    if argv.iter().any(|arg| arg == "--virtual-machines") {
        Some("virtualization".to_owned())
    } else if argv.iter().any(|arg| arg == "--open-updates") {
        Some("software/updates".to_owned())
    } else if argv.iter().any(|arg| arg == "--open-health") {
        // Notificação da verificação periódica: o relatório no Diagnóstico.
        Some("hardware/health".to_owned())
    } else if let Some(link) = argv.iter().find_map(|arg| arg.strip_prefix(LINK_SCHEME)) {
        let link = link.strip_suffix('/').unwrap_or(link);
        destination(link.split('/'))
    } else if let Some(page) = option_value(argv, "--page") {
        // A seção só vale dentro de uma aba.
        let tab = option_value(argv, "--tab");
        let section = tab.and(option_value(argv, "--section"));
        destination([Some(page), tab, section].into_iter().flatten())
    } else {
        None
    }
}

fn option_value<'a>(argv: &'a [String], name: &str) -> Option<&'a str> {
    let at = argv.iter().position(|arg| arg == name)?;
    argv.get(at + 1).map(String::as_str)
}

/// Junta os trechos se forem até três identificadores simples (minúsculas,
/// dígitos e hífen), como os ids de página, aba e seção da interface.
fn destination<'a>(parts: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let parts: Vec<&str> = parts.into_iter().collect();
    let valid = |part: &&str| {
        (1..=48).contains(&part.len())
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    (!parts.is_empty() && parts.len() <= 3 && parts.iter().all(valid)).then(|| parts.join("/"))
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
/// Abre a página de login de uma rede com portal cativo: só `http://host/`,
/// o endereço de checagem do NetworkManager que o teste `network` informa.
/// O portal só intercepta HTTP sem TLS, por isso o `open_uri` não serve.
#[tauri::command]
pub fn open_portal(uri: String) -> Result<(), String> {
    if !valid_portal_uri(&uri) {
        return Err("endereço não permitido".into());
    }
    let context = gio::AppLaunchContext::new();
    crate::webkit_env::clean_launch_context(&context);
    gio::AppInfo::launch_default_for_uri(&uri, Some(&context)).map_err(|e| e.to_string())
}

fn valid_portal_uri(uri: &str) -> bool {
    let Some(rest) = uri.strip_prefix("http://") else {
        return false;
    };
    let host = rest.strip_suffix('/').unwrap_or(rest);
    !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_uris_are_plain_hosts_only() {
        assert!(valid_portal_uri("http://conncheck.opensuse.org"));
        assert!(valid_portal_uri("http://conncheck.opensuse.org/"));
        for bad in [
            "https://x.org",
            "http://x.org/path",
            "http://user@x.org",
            "file:///etc",
            "http://",
            "javascript:alert(1)",
        ] {
            assert!(!valid_portal_uri(bad), "{bad}");
        }
    }

    fn page(args: &[&str]) -> Option<String> {
        let argv: Vec<String> = std::iter::once("vega-gtk")
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();
        requested_page(&argv)
    }

    #[test]
    fn page_options_build_the_destination() {
        assert_eq!(page(&[]), None);
        assert_eq!(
            page(&["--open-updates"]).as_deref(),
            Some("software/updates")
        );
        assert_eq!(page(&["--page", "network"]).as_deref(), Some("network"));
        assert_eq!(
            page(&[
                "--page",
                "software",
                "--tab",
                "developers",
                "--section",
                "databases"
            ])
            .as_deref(),
            Some("software/developers/databases")
        );
        // Seção sem aba não tem onde ficar.
        assert_eq!(
            page(&["--page", "software", "--section", "databases"]).as_deref(),
            Some("software")
        );
        assert_eq!(page(&["--page"]), None);
    }

    #[test]
    fn links_build_the_destination() {
        assert_eq!(
            page(&["vega://software/developers/databases"]).as_deref(),
            Some("software/developers/databases")
        );
        assert_eq!(page(&["vega://hardware/"]).as_deref(), Some("hardware"));
    }

    #[test]
    fn malformed_destinations_are_ignored() {
        for bad in [
            "vega://",
            "vega://software//databases",
            "vega://a/b/c/d",
            "vega://Software",
            "vega://software?x=1",
            "vega://software/../etc",
            "vega://software/<img>",
        ] {
            assert_eq!(page(&[bad]), None, "{bad}");
        }
        assert_eq!(page(&["--page", "x y"]), None);
        assert_eq!(page(&["--page", &"a".repeat(49)]), None);
    }
}
