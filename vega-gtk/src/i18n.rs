use gettextrs::{LocaleCategory, TextDomain};

const DOMAIN: &str = "vega-gtk";

/// Initializes gettext from the session's native message locale. Unsupported
/// locales are mapped to en-US before binding, making English the deterministic
/// fallback while keeping locale changes effective on the next launch.
pub fn init() {
    let locale = session_locale();
    // SAFETY: called once on the GTK main thread, before worker threads and
    // before any translated widget is created.
    unsafe { std::env::set_var("LANGUAGE", locale) };
    // Além dos caminhos padrão do sistema (/usr/share/locale, usado pelo
    // pacote instalado), procura também os .mo que o build.rs acabou de
    // gerar em `po/`, pra `cargo run` local funcionar sem instalar nada.
    let local_path = concat!(env!("CARGO_MANIFEST_DIR"), "/po");
    if let Err(error) = TextDomain::new("vega-gtk-fallback")
        .prepend(local_path)
        .locale_category(LocaleCategory::LcMessages)
        .init()
    {
        eprintln!("i18n: English fallback catalog unavailable: {error}");
    }
    let result = TextDomain::new(DOMAIN)
        .prepend(local_path)
        .locale_category(LocaleCategory::LcMessages)
        .init();
    if let Err(error) = result {
        eprintln!("i18n: falling back to source strings after catalog error: {error}");
    }
}

fn session_locale() -> &'static str {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .map_or("en-US", |value| normalize_locale(&value))
}

fn normalize_locale(value: &str) -> &'static str {
    let base = value.trim().split('@').next().unwrap_or("");
    let base = base.split('.').next().unwrap_or("").replace('_', "-");
    match base.to_ascii_lowercase().as_str() {
        "en-us" => "en_US",
        "pt-br" => "pt_BR",
        "es-es" => "es_ES",
        "zh-cn" => "zh_CN",
        _ => "en_US",
    }
}

/// Translates UI text and falls back to the complete English domain if the
/// active catalog does not contain this individual key.
pub fn gettext(message: &str) -> String {
    let translated = gettextrs::gettext(message);
    if translated == message {
        gettextrs::dgettext("vega-gtk-fallback", message)
    } else {
        translated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_does_not_panic_regardless_of_locale() {
        init();
    }

    #[test]
    fn locale_normalization_and_fallback() {
        for (input, expected) in [
            ("en_US.UTF-8", "en_US"),
            ("pt_BR.UTF-8", "pt_BR"),
            ("es_ES.UTF-8@custom", "es_ES"),
            ("zh_CN.UTF-8", "zh_CN"),
            ("fr_FR.UTF-8", "en_US"),
            ("../../pt_BR", "en_US"),
        ] {
            assert_eq!(normalize_locale(input), expected);
        }
    }
}
