//! Ferramentas instaladas por fora do Vega (Vega → Software, vitrines).
//!
//! Muitas ferramentas de terminal têm instalador próprio na conta do usuário
//! (o Claude Code em `~/.local/bin/claude`, o Codex por outro prefixo do npm,
//! o Terraform baixado à mão…). O pacote do cartão não está instalado, mas a
//! ferramenta está: o cartão mostra "Instalado" em vez de oferecer instalar
//! de novo. Procura cada comando no PATH do Vega e nas pastas pessoais de
//! executáveis, que o PATH de um app gráfico às vezes não inclui.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Nome de comando simples, sem caminho nem nada que o shell interprete.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.+".contains(c))
        && name != "."
        && name != ".."
}

fn executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Pastas onde procurar: o PATH, depois `~/.local/bin`, `~/bin` e `~/.cargo/bin`.
fn search_dirs(path: Option<&str>, home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = path
        .map(|path| std::env::split_paths(path).collect())
        .unwrap_or_default();
    if let Some(home) = home {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join("bin"));
        // rustup e cargo install, pelo instalador oficial do Rust.
        dirs.push(home.join(".cargo/bin"));
    }
    for dir in ["/usr/local/bin", "/usr/bin", "/bin"] {
        dirs.push(PathBuf::from(dir));
    }
    let mut seen = Vec::new();
    dirs.retain(|dir| dir.is_absolute() && !seen.contains(dir) && {
        seen.push(dir.clone());
        true
    });
    dirs
}

pub fn present_in(names: &[String], dirs: &[PathBuf]) -> Vec<String> {
    names
        .iter()
        .filter(|name| plain(name))
        .filter(|name| dirs.iter().any(|dir| executable(&dir.join(name))))
        .cloned()
        .collect()
}

#[tauri::command]
pub fn commands_present(names: Vec<String>) -> Vec<String> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let path = std::env::var("PATH").ok();
    present_in(&names, &search_dirs(path.as_deref(), home.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_executables_and_rejects_paths() {
        let root = std::env::temp_dir().join(format!("vega-cmds-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let tool = root.join("claude");
        std::fs::write(&tool, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(root.join("notes"), "").unwrap();
        let names = ["claude", "notes", "missing", "../claude", "/bin/sh"].map(String::from);
        assert_eq!(present_in(&names, std::slice::from_ref(&root)), ["claude"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn home_bin_dirs_are_searched_after_path() {
        let dirs = search_dirs(Some("/opt/x/bin:relative"), Some(Path::new("/home/u")));
        assert_eq!(dirs[0], PathBuf::from("/opt/x/bin"));
        assert!(dirs.contains(&PathBuf::from("/home/u/.local/bin")));
        assert!(!dirs.contains(&PathBuf::from("relative")));
    }
}
