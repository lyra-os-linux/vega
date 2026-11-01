//! Assistentes de IA de terminal (Vega → Software → Desenvolvedores).
//!
//! Codex CLI, Gemini CLI, GitHub Copilot CLI e opencode não têm RPM; a
//! documentação de cada um indica `npm install -g`. O Vega instala na conta do
//! usuário (`npm install -g --prefix ~/.local`), sem `sudo`, como a própria
//! Anthropic e o GitHub recomendam: os executáveis vão para `~/.local/bin` e o
//! assistente pode se atualizar sozinho sem pedir senha. O Node.js 24 + npm
//! vem dos repositórios oficiais (os assistentes pedem Node 22 ou mais).
//! Só os pacotes de `PACKAGES` são aceitos: a interface nunca passa texto
//! livre para o npm. O Claude Code vem do repositório RPM oficial da
//! Anthropic (pacote claude-code-repo), não daqui.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use gettextrs::gettext;
use tauri::AppHandle;

use super::flatpak_session::follow;

/// Pacotes npm que a vitrine oferece.
pub const PACKAGES: &[&str] = &[
    "@openai/codex",
    "@google/gemini-cli",
    "@github/copilot",
    "opencode-ai",
];

/// Ids próprios, acima dos do vegad, do Flatpak e dos apps do Windows.
static NEXT_ID: AtomicU32 = AtomicU32::new(0xD000_0000);

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
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
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
        follow(&app, tx, command, "npm", failed);
    });
    Ok(tx)
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
