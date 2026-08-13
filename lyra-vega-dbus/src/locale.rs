use std::process::Command;

pub const DEFAULT_LOCALE: &str = "en-US";

pub fn normalize_locale(value: &str) -> &'static str {
    let base = value
        .trim()
        .split_once('@')
        .map_or(value.trim(), |(base, _)| base)
        .split_once('.')
        .map_or_else(
            || value.trim().split('@').next().unwrap_or(""),
            |(base, _)| base,
        )
        .replace('_', "-");
    match base.to_ascii_lowercase().as_str() {
        "en-us" => "en-US",
        "pt-br" => "pt-BR",
        "es-es" => "es-ES",
        "zh-cn" => "zh-CN",
        _ => DEFAULT_LOCALE,
    }
}

pub fn current_locale() -> &'static str {
    resolve_locale(
        gnome_language().as_deref(),
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|name| {
                std::env::var(name)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .as_deref(),
    )
}

fn resolve_locale(gnome: Option<&str>, environment: Option<&str>) -> &'static str {
    gnome
        .or(environment)
        .map_or(DEFAULT_LOCALE, normalize_locale)
}

/// Reads the language selected in GNOME AccountsService. Command arguments are
/// passed directly (without a shell), and every failure falls back to the
/// session environment so headless and non-GNOME clients remain supported.
fn gnome_language() -> Option<String> {
    let username = std::env::var("SUDO_USER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("USER")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })?;
    let user_reply = Command::new("busctl")
        .args([
            "--system",
            "call",
            "org.freedesktop.Accounts",
            "/org/freedesktop/Accounts",
            "org.freedesktop.Accounts",
            "FindUserByName",
            "s",
            &username,
        ])
        .output()
        .ok()?;
    if !user_reply.status.success() {
        return None;
    }
    let path = quoted_value(&String::from_utf8(user_reply.stdout).ok()?)?;
    let language_reply = Command::new("busctl")
        .args([
            "--system",
            "get-property",
            "org.freedesktop.Accounts",
            &path,
            "org.freedesktop.Accounts.User",
            "Language",
        ])
        .output()
        .ok()?;
    if !language_reply.status.success() {
        return None;
    }
    quoted_value(&String::from_utf8(language_reply.stdout).ok()?)
        .filter(|value| !value.trim().is_empty())
}

fn quoted_value(output: &str) -> Option<String> {
    let (_, rest) = output.split_once('"')?;
    let (value, _) = rest.split_once('"')?;
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_and_fallback_are_stable() {
        for (input, expected) in [
            ("en_US.UTF-8", "en-US"),
            ("pt_BR.UTF-8", "pt-BR"),
            ("es_ES.UTF-8@custom", "es-ES"),
            ("zh_CN.UTF-8", "zh-CN"),
            ("", "en-US"),
            ("de_DE.UTF-8", "en-US"),
            ("../../pt_BR", "en-US"),
        ] {
            assert_eq!(normalize_locale(input), expected);
        }
    }

    #[test]
    fn gnome_language_takes_precedence_over_environment() {
        assert_eq!(
            resolve_locale(Some("es_ES.UTF-8"), Some("pt_BR.UTF-8")),
            "es-ES"
        );
        assert_eq!(resolve_locale(None, Some("zh_CN.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(None, None), DEFAULT_LOCALE);
    }

    #[test]
    fn accounts_service_values_are_parsed() {
        assert_eq!(
            quoted_value("o \"/org/freedesktop/Accounts/User1000\"\n").as_deref(),
            Some("/org/freedesktop/Accounts/User1000")
        );
        assert_eq!(
            quoted_value("s \"pt_BR.UTF-8\"\n").as_deref(),
            Some("pt_BR.UTF-8")
        );
    }
}
