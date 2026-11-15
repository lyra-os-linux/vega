//! Módulo de segurança dos bancos, o Warsaw (vega#210).
//!
//! Só lê o estado; não muda nada. A instalação e a remoção usam a transação de
//! pacote oficial da loja, e Ligar/Desligar usam `services_set_running`, que
//! passa pelo vegad. `SetServiceEnabled` não serve aqui: no Leap 16.1 o
//! `/etc/init.d/warsaw` do fabricante faz `systemctl enable/disable` falhar,
//! e o Warsaw só deve rodar enquanto o usuário usa o banco (warsaw#4).
//!
//! O Warsaw instala em `/usr/local`, que fica fora dos snapshots do Snapper.
//! Depois de um rollback o RPM e os arquivos podem discordar (warsaw#12): o
//! pacote consta como instalado sem os arquivos, ou os arquivos sobram sem o
//! pacote. Os dois casos aparecem aqui para o cartão oferecer "Reparar", que
//! reinstala o pacote.

use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde::Serialize;

pub const PACKAGE: &str = "warsaw";
pub const UNIT: &str = "warsaw.service";
/// Porta local em que o `core` atende o site do banco.
const PORT: u16 = 30900;
/// Diretórios privados do Warsaw em `/usr/local`.
const PRIVATE_DIRS: [&str; 3] = ["bin/warsaw", "etc/warsaw", "lib/warsaw"];

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Health {
    /// Não instalado e sem restos em `/usr/local`.
    Absent,
    /// Não instalado, mas com arquivos do Warsaw em `/usr/local`.
    Leftovers,
    /// Instalado, mas faltam arquivos ou eles não batem com o RPM.
    Broken,
    /// Instalado e desligado.
    Off,
    /// O serviço tenta subir sem conseguir (`activating` ou `failed`).
    Stuck,
    /// Ligado, mas ainda sem atender na porta local.
    Starting,
    /// Ligado e atendendo na porta local.
    Ready,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct BankModule {
    pub health: Health,
    pub version: Option<String>,
    /// Linhas do `rpm -V` que tornam a instalação incoerente.
    pub problems: Vec<String>,
}

/// Linhas do `rpm -V` que indicam arquivo faltando ou diferente do pacote.
/// Os arquivos de configuração (`c`) podem mudar com o uso e não contam.
pub fn verify_problems(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .filter(|line| {
            let mut fields = line.split_whitespace();
            let flags = fields.next().unwrap_or_default();
            let kind = fields.next().unwrap_or_default();
            flags == "missing" || kind != "c"
        })
        .map(|line| line.trim().to_owned())
        .collect()
}

pub fn classify(
    version: Option<String>,
    problems: Vec<String>,
    leftovers: bool,
    active_state: &str,
    listening: bool,
) -> BankModule {
    let health = match &version {
        None if leftovers => Health::Leftovers,
        None => Health::Absent,
        Some(_) if !problems.is_empty() => Health::Broken,
        Some(_) => match active_state {
            "active" if listening => Health::Ready,
            "active" | "reloading" => Health::Starting,
            "activating" | "failed" => Health::Stuck,
            _ => Health::Off,
        },
    };
    BankModule {
        health,
        version,
        problems,
    }
}

fn run(program: &str, args: &[&str]) -> Option<(bool, String)> {
    // Saída em inglês: com o sistema em pt-BR, o `rpm -V` escreve "faltando"
    // no fim da linha em vez de "missing" no começo.
    let output = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

fn installed_version() -> Option<String> {
    match run("rpm", &["-q", "--qf", "%{VERSION}", PACKAGE]) {
        Some((true, version)) if !version.trim().is_empty() => Some(version.trim().to_owned()),
        _ => None,
    }
}

fn has_leftovers(prefix: &Path) -> bool {
    PRIVATE_DIRS.iter().any(|dir| prefix.join(dir).exists())
}

fn listening() -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], PORT));
    TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
}

/// Estado do Warsaw nesta máquina. Roda sem root: `rpm -V` lê arquivos que
/// qualquer usuário pode ler, e `systemctl is-active` não pede autorização.
#[tauri::command]
pub fn bank_module_status() -> BankModule {
    let version = installed_version();
    let problems = if version.is_some() {
        run("rpm", &["-V", PACKAGE])
            .map(|(_, output)| verify_problems(&output))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let active_state = run("systemctl", &["is-active", UNIT])
        .map(|(_, state)| state.trim().to_owned())
        .unwrap_or_default();
    let listening = active_state == "active" && listening();
    classify(
        version,
        problems,
        has_leftovers(Path::new("/usr/local")),
        &active_state,
        listening,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version() -> Option<String> {
        Some("2.21.4".to_owned())
    }

    #[test]
    fn missing_files_and_changed_binaries_count_but_config_does_not() {
        let output = "missing     /usr/local/bin/warsaw/core\n\
                      S.5....T.  c /usr/local/etc/warsaw/gas.dbd\n\
                      S.5....T.    /usr/local/lib/warsaw/wsbrmu.so\n\n";
        assert_eq!(
            verify_problems(output),
            vec![
                "missing     /usr/local/bin/warsaw/core".to_owned(),
                "S.5....T.    /usr/local/lib/warsaw/wsbrmu.so".to_owned(),
            ]
        );
        assert!(verify_problems("").is_empty());
    }

    #[test]
    fn rollback_after_install_leaves_files_without_the_package() {
        assert_eq!(
            classify(None, Vec::new(), true, "inactive", false).health,
            Health::Leftovers
        );
        assert_eq!(
            classify(None, Vec::new(), false, "inactive", false).health,
            Health::Absent
        );
    }

    #[test]
    fn rollback_after_removal_leaves_the_package_without_files() {
        let problems = vec!["missing     /usr/local/bin/warsaw/core".to_owned()];
        // O serviço preso em "activating" não esconde a causa: faltam arquivos.
        assert_eq!(
            classify(version(), problems, false, "activating", false).health,
            Health::Broken
        );
    }

    #[test]
    fn service_states_map_to_the_card() {
        let state = |active: &str, listening: bool| {
            classify(version(), Vec::new(), false, active, listening).health
        };
        assert_eq!(state("inactive", false), Health::Off);
        assert_eq!(state("active", true), Health::Ready);
        assert_eq!(state("active", false), Health::Starting);
        assert_eq!(state("activating", false), Health::Stuck);
        assert_eq!(state("failed", false), Health::Stuck);
    }

    #[test]
    fn leftovers_are_the_private_directories_under_usr_local() {
        let root = std::env::temp_dir().join(format!("vega-banks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("share/fonts")).unwrap();
        assert!(!has_leftovers(&root));
        std::fs::create_dir_all(root.join("lib/warsaw")).unwrap();
        assert!(has_leftovers(&root));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
