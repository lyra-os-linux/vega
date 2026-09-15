use gettextrs::{LocaleCategory, TextDomain};

const DOMAIN: &str = "vega-gtk";

/// Called before GTK/GIO create worker threads. Follow the process message
/// locale, or the explicit per-app preference; never modify AccountsService.
pub fn init(preference: &str) {
    let language = std::env::var("LANGUAGE").unwrap_or_default();
    let environment = ["LC_ALL", "LC_MESSAGES", "LANG"].map(std::env::var);
    let locale = resolve_locale(
        preference,
        &language,
        environment.iter().filter_map(|value| value.as_deref().ok()),
    );
    // Preserve regional formatting but keep GTK from resetting LC_MESSAGES
    // after the app's language preference has been applied.
    gtk::disable_setlocale();
    if gettextrs::setlocale(LocaleCategory::LcAll, "").is_none() {
        gettextrs::setlocale(LocaleCategory::LcAll, "C.UTF-8");
    }
    init_locale(locale);
}

fn init_locale(locale: &str) {
    // SAFETY: initialization precedes GTK/GIO and all application threads.
    unsafe { std::env::set_var("LANGUAGE", locale) };
    // glibc-locale-base supplies en_US.UTF-8. Gettext suppresses translations
    // in C/C.UTF-8, so use this real locale when the chosen regional data is
    // absent. LANGUAGE still selects the app's Portuguese/English/Spanish MO.
    gettextrs::setlocale(LocaleCategory::LcMessages, "en_US.UTF-8");
    let mut domain = TextDomain::new(DOMAIN)
        .locale(&format!("{locale}.UTF-8"))
        .locale_category(LocaleCategory::LcMessages);
    // Installed binaries must use their packaged catalogs, even when this
    // machine happens to retain the build checkout. Cargo dev/tests may use po/.
    let installed = std::env::current_exe()
        .ok()
        .is_some_and(|path| path.parent() == Some(std::path::Path::new("/usr/bin")));
    if cfg!(debug_assertions) && !installed {
        domain = domain.prepend(concat!(env!("CARGO_MANIFEST_DIR"), "/po"));
    }
    if let Err(error) = domain.init() {
        eprintln!("i18n: falling back to source strings after catalog error: {error}");
    }
}

fn resolve_locale<'a>(
    preference: &str,
    language: &str,
    environment: impl IntoIterator<Item = &'a str>,
) -> &'static str {
    if let Some(locale) = supported_locale(preference) {
        return locale;
    }
    if !language.trim().is_empty() {
        return language
            .split(':')
            .find_map(supported_locale)
            .unwrap_or("en_US");
    }
    environment
        .into_iter()
        .find(|value| !value.trim().is_empty() && !is_portable_locale(value))
        .map_or("en_US", normalize_locale)
}

// Retain the portable-launcher fallback: a C/POSIX LC_ALL does not hide a
// meaningful LANG supplied by the desktop. Without one, use English.
fn is_portable_locale(value: &str) -> bool {
    let base = value.trim().split('@').next().unwrap_or("");
    let base = base.split('.').next().unwrap_or("");
    base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX")
}

fn supported_locale(value: &str) -> Option<&'static str> {
    let base = value.trim().split('@').next().unwrap_or("");
    let base = base.split('.').next().unwrap_or("").replace('_', "-");
    match base.to_ascii_lowercase().split('-').next().unwrap_or("") {
        "en" => Some("en_US"),
        "pt" => Some("pt_BR"),
        "es" => Some("es_ES"),
        _ => None,
    }
}

fn normalize_locale(value: &str) -> &'static str {
    supported_locale(value).unwrap_or("en_US")
}

pub fn gettext(message: &str) -> String {
    gettextrs::gettext(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_normalization_and_fallback() {
        for (input, expected) in [
            ("en_US.UTF-8", "en_US"),
            ("pt_BR.UTF-8", "pt_BR"),
            ("es_ES.UTF-8@custom", "es_ES"),
            ("zh_CN.UTF-8", "en_US"),
            ("fr_FR.UTF-8", "en_US"),
            ("../../pt_BR", "en_US"),
        ] {
            assert_eq!(normalize_locale(input), expected);
        }
    }

    #[test]
    fn explicit_choice_and_process_language_have_predictable_precedence() {
        assert_eq!(resolve_locale("pt_BR", "en_US", ["es_ES.UTF-8"]), "pt_BR");
        assert_eq!(
            resolve_locale("system", "fr_FR:es_MX:en", ["pt_BR.UTF-8"]),
            "es_ES"
        );
        assert_eq!(resolve_locale("system", "fr_FR", ["pt_BR.UTF-8"]), "en_US");
        assert_eq!(
            resolve_locale("system", "", ["en_GB.UTF-8", "pt_BR.UTF-8"]),
            "en_US"
        );
        assert_eq!(
            resolve_locale("system", "", ["", "es_ES.UTF-8", "pt_BR.UTF-8"]),
            "es_ES"
        );
        assert_eq!(
            resolve_locale("invalid-setting", "", ["pt_PT.UTF-8"]),
            "pt_BR"
        );
        assert_eq!(resolve_locale("system", "", ["zh_CN.UTF-8"]), "en_US");
        assert_eq!(resolve_locale("system", "", []), "en_US");
    }

    #[test]
    fn portable_locale_does_not_hide_desktop_language() {
        assert_eq!(
            resolve_locale("system", "", ["C.UTF-8", "C.UTF-8", "pt_BR.UTF-8"]),
            "pt_BR"
        );
        assert_eq!(
            resolve_locale("system", "", ["POSIX", "es_ES.UTF-8"]),
            "es_ES"
        );
        assert_eq!(resolve_locale("system", "", ["C.UTF-8", "POSIX"]), "en_US");
    }

    /// Runs in a dedicated process because gettext's locale and active domain
    /// are process-global state and Rust executes unit tests concurrently.
    #[test]
    fn translation_subprocess() {
        let Ok(locale) = std::env::var("VEGA_GTK_TEST_LOCALE") else {
            return;
        };
        let expected = std::env::var("VEGA_GTK_TEST_EXPECTED").unwrap();
        init_locale(&locale);
        assert_eq!(gettext("Painel"), expected);
        let clearing = match locale.as_str() {
            "en_US" => "Clearing conversation…",
            "es_ES" => "Borrando conversación…",
            "pt_BR" => "Limpando conversa…",
            other => panic!("unexpected test locale: {other}"),
        };
        assert_eq!(gettext("Limpando conversa…"), clearing);
        let (owner, timeout) = match locale.as_str() {
            "en_US" => (
                "The software service stopped or restarted.",
                "The monitoring deadline expired.",
            ),
            "es_ES" => (
                "El servicio de software se detuvo o se reinició.",
                "Se agotó el plazo de seguimiento.",
            ),
            "pt_BR" => (
                "O serviço de software foi interrompido ou reiniciado.",
                "O prazo de acompanhamento terminou.",
            ),
            other => panic!("unexpected test locale: {other}"),
        };
        assert!(
            lyra_vega_dbus::SoftwareClientError::ServiceOwnerChanged
                .to_string()
                .starts_with(owner)
        );
        assert!(
            lyra_vega_dbus::SoftwareClientError::TransactionTimedOut
                .to_string()
                .starts_with(timeout)
        );
    }

    #[test]
    fn loads_each_catalog_even_when_parent_locale_is_portable() {
        let executable = std::env::current_exe().unwrap();
        for (locale, expected) in [
            ("en_US", "Dashboard"),
            ("pt_BR", "Painel"),
            ("es_ES", "Panel de control"),
        ] {
            let status = std::process::Command::new(&executable)
                .arg("--exact")
                .arg("i18n::tests::translation_subprocess")
                .arg("--nocapture")
                .env("LC_ALL", "C.UTF-8")
                .env("LC_MESSAGES", "C.UTF-8")
                .env("LANG", "C.UTF-8")
                .env("VEGA_GTK_TEST_LOCALE", locale)
                .env("VEGA_GTK_TEST_EXPECTED", expected)
                .status()
                .unwrap();
            assert!(status.success(), "failed to load {locale} catalog");
        }
    }
}
