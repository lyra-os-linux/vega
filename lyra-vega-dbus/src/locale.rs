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
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .map_or(DEFAULT_LOCALE, |value| normalize_locale(&value))
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
}
