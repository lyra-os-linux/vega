//! Assistentes de IA de terminal (Vega → Software → Desenvolvedores).
//!
//! Codex CLI, Gemini CLI, GitHub Copilot CLI, opencode, o Mermaid CLI e o
//! Zowe CLI não têm RPM; a
//! documentação de cada um indica `npm install -g`. O Vega instala na conta do
//! usuário (`npm install -g --prefix ~/.local`), sem `sudo`, como a própria
//! Anthropic e o GitHub recomendam: os executáveis vão para `~/.local/bin` e o
//! assistente pode se atualizar sozinho sem pedir senha. O Node.js 24 + npm
//! vem dos repositórios oficiais (os assistentes pedem Node 22 ou mais).
//! Só os pacotes de `PACKAGES` são aceitos: a interface nunca passa texto
//! livre para o npm. O Claude Code vem do repositório RPM oficial da
//! Anthropic (pacote claude-code-repo), não daqui.
//!
//! O shell padrão do Lyra é o fish, que não põe `~/.local/bin` no PATH
//! sozinho (o `/etc/profile` só vale para o bash, e só se a pasta já existia
//! no login). Depois de instalar, o Vega roda `fish_add_path ~/.local/bin`,
//! que grava em `fish_user_paths` e não faz nada se a pasta já estiver lá.

use std::path::{Path, PathBuf};
use std::process::Command;

use gettextrs::gettext;
use tauri::AppHandle;

use super::flatpak_session::follow_then;

/// Pacotes npm que a vitrine oferece.
pub const PACKAGES: &[&str] = &[
    "@openai/codex",
    "@google/gemini-cli",
    "@github/copilot",
    "opencode-ai",
    "@mermaid-js/mermaid-cli",
    "@zowe/cli",
    "@salesforce/cli",
];

/// `~/.local`, o prefixo do npm na conta do usuário.
fn prefix() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local"))
}

/// Pacotes da lista instalados sob `prefix` (`lib/node_modules/<pacote>`).
pub fn installed_under(prefix: &Path) -> Vec<String> {
    PACKAGES
        .iter()
        .filter(|package| {
            prefix
                .join("lib/node_modules")
                .join(package)
                .join("package.json")
                .is_file()
        })
        .map(|package| (*package).to_owned())
        .collect()
}

#[tauri::command]
pub fn npm_installed() -> Vec<String> {
    prefix()
        .map(|prefix| installed_under(&prefix))
        .unwrap_or_default()
}

/// Argumentos do npm para instalar (sempre a última versão) ou remover.
pub fn arguments(install: bool, package: &str, prefix: &Path) -> Vec<String> {
    let mut args: Vec<String> = if install {
        vec!["install".into(), "--global".into()]
    } else {
        vec!["uninstall".into(), "--global".into()]
    };
    args.extend([
        "--prefix".into(),
        prefix.display().to_string(),
        "--no-fund".into(),
        "--no-audit".into(),
    ]);
    args.push(if install {
        format!("{package}@latest")
    } else {
        package.to_owned()
    });
    args
}

fn start(app: AppHandle, install: bool, package: String) -> Result<u32, String> {
    if !PACKAGES.contains(&package.as_str()) {
        return Err(gettext("Ferramenta desconhecida."));
    }
    let prefix = prefix().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let mut command = Command::new("npm");
        command
            .args(arguments(install, &package, &prefix))
            .env("npm_config_update_notifier", "false")
            .env("npm_config_loglevel", "warn");
        crate::webkit_env::clean_command(&mut command);
        let failed = if install {
            gettext("A instalação pelo npm falhou.")
        } else {
            gettext("A remoção pelo npm falhou.")
        };
        let bin = prefix.join("bin");
        follow_then(&app, tx, command, "npm", failed, |success| {
            if success && install {
                add_to_fish_path(&bin);
            }
        });
    });
    Ok(tx)
}

/// Comando do fish que põe `dir` no PATH de todas as sessões do usuário.
pub fn fish_add_path_command(dir: &Path) -> Option<String> {
    let dir = dir.to_str()?;
    // Só caminhos absolutos simples: o texto vai para dentro de um comando fish.
    let plain = dir.starts_with('/')
        && dir
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+@".contains(c));
    plain.then(|| format!("fish_add_path --universal {dir}"))
}

/// Melhor esforço: sem fish instalado, ou se falhar, segue sem erro (o bash
/// já lê `~/.local/bin` pelo /etc/profile no próximo login).
fn add_to_fish_path(dir: &Path) {
    let Some(script) = fish_add_path_command(dir) else {
        return;
    };
    // Sem `--no-config`: com ele o fish 4 não grava variáveis universais.
    let mut command = Command::new("fish");
    command
        .args(["-c", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::webkit_env::clean_command(&mut command);
    let _ = command.status();
}

/// Pacotes da lista com versão mais nova no npm, a partir do JSON de
/// `npm outdated --json`: `(pacote, instalada, mais nova)`.
pub fn parse_outdated(raw: &str) -> Vec<(String, String, String)> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(raw) else {
        return Vec::new();
    };
    PACKAGES
        .iter()
        .filter_map(|package| {
            let entry = map.get(*package)?;
            let current = entry.get("current")?.as_str()?;
            let latest = entry.get("latest")?.as_str()?;
            (current != latest)
                .then(|| ((*package).to_owned(), current.to_owned(), latest.to_owned()))
        })
        .collect()
}

/// Atualizações dos assistentes instalados pelo Vega, para a lista de
/// Atualizações do Software (vega#305). Consulta o registro do npm; sem npm,
/// sem rede ou sem nada instalado, devolve a lista vazia.
#[tauri::command]
pub async fn npm_updates() -> Vec<super::software::Package> {
    tauri::async_runtime::spawn_blocking(|| {
        let prefix = prefix()?;
        if installed_under(&prefix).is_empty() {
            return None;
        }
        let mut command = Command::new("npm");
        command
            .args(["outdated", "--global", "--json", "--prefix"])
            .arg(&prefix)
            .current_dir(&prefix)
            .env("npm_config_update_notifier", "false")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::webkit_env::clean_command(&mut command);
        // `npm outdated` sai com 1 quando há atualização: vale a saída.
        let output = command.output().ok()?;
        let raw = String::from_utf8_lossy(&output.stdout);
        Some(
            parse_outdated(&raw)
                .into_iter()
                .map(|(id, current, latest)| super::software::Package {
                    origin: "npm".into(),
                    name: id.clone(),
                    id,
                    description: format!("{current} → {latest}"),
                    installed: true,
                    icon: String::new(),
                    repository: "npm".into(),
                    icon_data: None,
                })
                .collect(),
        )
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}

#[tauri::command]
pub fn npm_install(app: AppHandle, package: String) -> Result<u32, String> {
    start(app, true, package)
}

#[tauri::command]
pub fn npm_remove(app: AppHandle, package: String) -> Result<u32, String> {
    start(app, false, package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_install_latest_into_the_user_prefix() {
        let prefix = Path::new("/home/u/.local");
        assert_eq!(
            arguments(true, "@openai/codex", prefix),
            [
                "install",
                "--global",
                "--prefix",
                "/home/u/.local",
                "--no-fund",
                "--no-audit",
                "@openai/codex@latest"
            ]
        );
        assert_eq!(
            arguments(false, "opencode-ai", prefix),
            [
                "uninstall",
                "--global",
                "--prefix",
                "/home/u/.local",
                "--no-fund",
                "--no-audit",
                "opencode-ai"
            ]
        );
    }

    #[test]
    fn outdated_keeps_only_listed_packages_with_a_newer_version() {
        let raw = r#"{
            "@openai/codex": {"current": "0.169.0", "wanted": "0.171.0", "latest": "0.171.0"},
            "opencode-ai": {"current": "1.0.0", "wanted": "1.0.0", "latest": "1.0.0"},
            "left-pad": {"current": "1.0.0", "latest": "2.0.0"}
        }"#;
        assert_eq!(
            parse_outdated(raw),
            vec![(
                "@openai/codex".to_owned(),
                "0.169.0".to_owned(),
                "0.171.0".to_owned()
            )]
        );
        assert!(parse_outdated("{}").is_empty());
        assert!(parse_outdated("not json").is_empty());
    }

    #[test]
    fn installed_reads_the_user_node_modules() {
        let root = std::env::temp_dir().join(format!("vega-npm-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let codex = root.join("lib/node_modules/@openai/codex");
        std::fs::create_dir_all(&codex).unwrap();
        std::fs::write(codex.join("package.json"), "{}").unwrap();
        std::fs::create_dir_all(root.join("lib/node_modules/outro")).unwrap();
        assert_eq!(installed_under(&root), ["@openai/codex"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn fish_path_command_only_takes_plain_absolute_paths() {
        assert_eq!(
            fish_add_path_command(Path::new("/home/u/.local/bin")).as_deref(),
            Some("fish_add_path --universal /home/u/.local/bin")
        );
        assert_eq!(fish_add_path_command(Path::new("relative/bin")), None);
        assert_eq!(
            fish_add_path_command(Path::new("/home/u;rm -rf ~/bin")),
            None
        );
        assert_eq!(fish_add_path_command(Path::new("/home/my user/bin")), None);
    }

    #[test]
    fn packages_are_plain_npm_names() {
        for package in PACKAGES {
            assert!(
                package
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "@/-".contains(c)),
                "{package}"
            );
            assert!(!package.starts_with('-'), "{package}");
        }
    }
}
