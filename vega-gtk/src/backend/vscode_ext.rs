//! Extensões do VS Code (Vega → Software → Desenvolvedores → COBOL e
//! mainframe).
//!
//! O Vega instala a extensão na conta do usuário com o próprio VS Code
//! (`code --install-extension`), sem `sudo`: ela vai para `~/.vscode/extensions`
//! e o VS Code a atualiza sozinho. Só as extensões de `EXTENSIONS` são aceitas:
//! a interface nunca passa texto livre para o `code`. O VS Code vem do
//! repositório oficial da Microsoft (pacote vscode-repo); sem ele, a interface
//! oferece instalá-lo antes.
//!
//! O registro de instaladas é `~/.vscode/extensions/extensions.json`, que o
//! VS Code mantém. A pasta da extensão não serve: ao remover, o VS Code só a
//! marca como obsoleta e a apaga mais tarde.

use std::path::{Path, PathBuf};
use std::process::Command;

use gettextrs::gettext;
use tauri::AppHandle;

use super::flatpak_session::follow_then;

/// Extensões que a vitrine oferece, no formato `editor.nome` do Marketplace.
pub const EXTENSIONS: &[&str] = &[
    "broadcomMFD.code4z-extension-pack",
    "Zowe.vscode-extension-for-zowe",
    "Oracle.sql-developer",
    "Oracle.oracledevtools",
    "salesforce.salesforcedx-vscode",
];

/// `~/.vscode/extensions/extensions.json`.
fn registry() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".vscode/extensions/extensions.json"))
}

/// Extensões da lista presentes no registro do VS Code (o id do registro vem
/// em minúsculas; a comparação ignora maiúsculas).
pub fn installed_in(text: &str) -> Vec<String> {
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str(text) else {
        return Vec::new();
    };
    let ids: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.pointer("/identifier/id")?.as_str())
        .map(str::to_ascii_lowercase)
        .collect();
    EXTENSIONS
        .iter()
        .filter(|extension| ids.contains(&extension.to_ascii_lowercase()))
        .map(|extension| (*extension).to_owned())
        .collect()
}

pub fn installed_at(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| installed_in(&text))
        .unwrap_or_default()
}

#[tauri::command]
pub fn vscode_extensions_installed() -> Vec<String> {
    registry()
        .map(|path| installed_at(&path))
        .unwrap_or_default()
}

/// Argumentos do `code` para instalar (sempre a última versão) ou remover.
pub fn arguments(install: bool, extension: &str) -> Vec<String> {
    if install {
        vec![
            "--install-extension".into(),
            extension.to_owned(),
            "--force".into(),
        ]
    } else {
        vec!["--uninstall-extension".into(), extension.to_owned()]
    }
}

fn start(app: AppHandle, install: bool, extension: String) -> Result<u32, String> {
    if !EXTENSIONS.contains(&extension.as_str()) {
        return Err(gettext("Extensão desconhecida."));
    }
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let mut command = Command::new("code");
        command.args(arguments(install, &extension));
        crate::webkit_env::clean_command(&mut command);
        let failed = if install {
            gettext("A instalação da extensão pelo VS Code falhou.")
        } else {
            gettext("A remoção da extensão pelo VS Code falhou.")
        };
        follow_then(&app, tx, command, "code", failed, |_| {});
    });
    Ok(tx)
}

#[tauri::command]
pub fn vscode_extension_install(app: AppHandle, extension: String) -> Result<u32, String> {
    start(app, true, extension)
}

#[tauri::command]
pub fn vscode_extension_remove(app: AppHandle, extension: String) -> Result<u32, String> {
    start(app, false, extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_install_latest_and_remove() {
        assert_eq!(
            arguments(true, "broadcomMFD.code4z-extension-pack"),
            [
                "--install-extension",
                "broadcomMFD.code4z-extension-pack",
                "--force"
            ]
        );
        assert_eq!(
            arguments(false, "broadcomMFD.code4z-extension-pack"),
            ["--uninstall-extension", "broadcomMFD.code4z-extension-pack"]
        );
    }

    #[test]
    fn installed_reads_the_registry_ignoring_case() {
        let text = r#"[
            {"identifier": {"id": "broadcommfd.code4z-extension-pack"}, "version": "2.2.1"},
            {"identifier": {"id": "ms-python.python"}, "version": "1.0.0"}
        ]"#;
        assert_eq!(installed_in(text), ["broadcomMFD.code4z-extension-pack"]);
        assert!(installed_in("[]").is_empty());
        assert!(installed_in("não é json").is_empty());
    }

    #[test]
    fn extensions_are_plain_marketplace_ids() {
        for extension in EXTENSIONS {
            let (publisher, name) = extension.split_once('.').expect(extension);
            for part in [publisher, name] {
                assert!(!part.is_empty() && !part.starts_with('-'), "{extension}");
                assert!(
                    part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                    "{extension}"
                );
            }
        }
    }
}
