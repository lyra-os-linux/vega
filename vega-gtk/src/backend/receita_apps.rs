//! Programas da Receita Federal na conta do usuário (vega#212).
//!
//! A Receita publica o IRPF para Linux sem pedir login, como instalador
//! install4j que já traz o próprio Java. O Vega baixa a versão conferida pelo
//! Lyra, confere tamanho e SHA-256 (a Receita não publica checksum) com a
//! conferência do download guiado e roda o instalador em modo silencioso, sem
//! root, em `~/ProgramasRFB/<programa>`, a pasta padrão da Receita. Uma
//! versão nova da Receita só é instalada depois que o Lyra confere o hash.
//!
//! Os atalhos do menu são os que o próprio instalador cria
//! (`install4j_*.desktop`). Na remoção, as declarações, que ficam em
//! `aplicacao/dados` dentro da pasta do programa, são movidas antes para
//! `~/ProgramasRFB/<programa>-declarações`; nada do contribuinte é apagado.

use std::io::Write;
use std::path::{Path, PathBuf};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::guided_download::{self, Expected};
use super::software::TransactionEvent;

/// Programa da Receita com o instalador conferido pelo Lyra.
#[derive(Debug, Clone, Copy)]
pub struct Program {
    pub id: &'static str,
    pub name: &'static str,
    /// Pasta em `~/ProgramasRFB` e nome do executável dentro dela.
    pub dir: &'static str,
    pub url: &'static str,
    pub installer: Expected,
}

pub const PROGRAMS: &[Program] = &[
    Program {
        id: "irpf2026",
        name: "IRPF 2026",
        dir: "IRPF2026",
        url: "https://downloadirpf.receita.fazenda.gov.br/irpf/2026/irpf/arquivos/IRPF2026Linux-x86_64v1.5.sh.bin",
        installer: Expected {
            page: "https://www.gov.br/receitafederal/pt-br/centrais-de-conteudo/download/pgd/dirpf",
            file: "IRPF2026Linux-x86_64v1.5.sh.bin",
            size: 110_149_481,
            sha256: "fada62389c323bee32129de26e5769367f24dfb4effb0243d2fc690d50ea08e2",
            version: "1.5",
        },
    },
    // Ano anterior, para retificar a declaração já entregue.
    Program {
        id: "irpf2025",
        name: "IRPF 2025",
        dir: "IRPF2025",
        url: "https://downloadirpf.receita.fazenda.gov.br/irpf/2025/irpf/arquivos/IRPF2025Linux-x86_64v1.8.sh.bin",
        installer: Expected {
            page: "https://www.gov.br/receitafederal/pt-br/centrais-de-conteudo/download/pgd/dirpf",
            file: "IRPF2025Linux-x86_64v1.8.sh.bin",
            size: 87_854_766,
            sha256: "e5812fe4246ef0bc5e75f2a727ccae399ab3342a148d2b62abd125100f499856",
            version: "1.8",
        },
    },
];

pub fn program(id: &str) -> Option<&'static Program> {
    PROGRAMS.iter().find(|program| program.id == id)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".local/share")))
}

fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".cache")))
        .map(|cache| cache.join("vega"))
}

pub fn install_dir(home: &Path, program: &Program) -> PathBuf {
    home.join("ProgramasRFB").join(program.dir)
}

fn launcher(home: &Path, program: &Program) -> PathBuf {
    install_dir(home, program).join(program.dir)
}

/// Atalho que o vega-gtk 5.17.0 de desenvolvimento criava; o instalador da
/// Receita já cria os dele, então este só é apagado.
fn legacy_desktop_path(data: &Path, program: &Program) -> PathBuf {
    data.join(format!("applications/lyra-receita-{}.desktop", program.id))
}

/// Pasta onde as declarações ficam guardadas depois da remoção. Se já existir
/// (de uma remoção anterior), ganha um número no fim.
pub fn declarations_target(home: &Path, program: &Program) -> PathBuf {
    let base = home
        .join("ProgramasRFB")
        .join(format!("{}-declarações", program.dir));
    let mut target = base.clone();
    let mut n = 2;
    while target.exists() {
        target = PathBuf::from(format!("{} ({n})", base.display()));
        n += 1;
    }
    target
}

/// Atalhos do install4j que apontam para a pasta do programa.
fn installer_desktop_files(data: &Path, dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(data.join("applications")) else {
        return Vec::new();
    };
    let needle = format!("{}/", dir.display());
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("install4j_") && name.ends_with(".desktop"))
                && std::fs::read_to_string(path).is_ok_and(|text| text.contains(&needle))
        })
        .collect()
}

fn download(url: &str, target: &Path) -> Result<(), String> {
    let mut response = reqwest::blocking::Client::builder()
        .https_only(true)
        .user_agent("lyra-vega")
        .timeout(std::time::Duration::from_secs(1800))
        .build()
        .and_then(|client| client.get(url).send())
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::File::create(target).map_err(|error| error.to_string())?;
    response
        .copy_to(&mut file)
        .map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())
}

fn install(program: &Program, report: &dyn Fn(u32, String)) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let cache = cache_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    if std::env::consts::ARCH != "x86_64" {
        return Err(gettext("A Receita só publica este programa para x86_64."));
    }
    std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let installer = cache.join(program.installer.file);
    report(
        5,
        gettext("Baixando {file} do site da Receita Federal…")
            .replace("{file}", program.installer.file),
    );
    let result = download(program.url, &installer)
        .and_then(|()| {
            report(60, gettext("Conferindo o instalador…"));
            guided_download::verify(&program.installer, &installer)
                .map_err(|rejection| rejection.message(&program.installer))
        })
        .and_then(|()| {
            report(
                70,
                gettext("Instalando {name}…").replace("{name}", program.name),
            );
            let mut command = std::process::Command::new("sh");
            command
                .arg(&installer)
                .args(["-q", "-overwrite", "-dir"])
                .arg(install_dir(&home, program));
            crate::webkit_env::clean_command(&mut command);
            let status = command.status().map_err(|error| error.to_string())?;
            if !status.success() || !launcher(&home, program).is_file() {
                return Err(gettext("O instalador da Receita falhou ({status}).")
                    .replace("{status}", &status.to_string()));
            }
            Ok(())
        });
    let _ = std::fs::remove_file(&installer);
    let _ = std::fs::remove_file(legacy_desktop_path(&data, program));
    result
}

/// Guarda as declarações e apaga o programa e os atalhos dele. Devolve a
/// pasta das declarações, quando havia alguma.
fn remove(program: &Program) -> Result<Option<PathBuf>, String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let dir = install_dir(&home, program);
    let dados = dir.join("aplicacao/dados");
    let kept = if dados.is_dir() {
        let target = declarations_target(&home, program);
        // Mesma pasta pessoal: rename não copia, e falhar aqui interrompe a
        // remoção antes de apagar qualquer coisa.
        std::fs::rename(&dados, &target).map_err(|error| error.to_string())?;
        Some(target)
    } else {
        None
    };
    for entry in installer_desktop_files(&data, &dir) {
        let _ = std::fs::remove_file(entry);
    }
    let _ = std::fs::remove_file(legacy_desktop_path(&data, program));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
    }
    Ok(kept)
}

fn refresh_menu() {
    if let Some(data) = data_dir() {
        let mut command = std::process::Command::new("update-desktop-database");
        command.arg(data.join("applications"));
        crate::webkit_env::clean_command(&mut command);
        let _ = command.status();
    }
}

/// Programas instalados, pelo Vega ou à mão, na pasta padrão da Receita.
#[tauri::command]
pub fn receita_installed() -> Vec<String> {
    let Some(home) = home() else {
        return Vec::new();
    };
    PROGRAMS
        .iter()
        .filter(|program| launcher(&home, program).is_file())
        .map(|program| program.id.to_owned())
        .collect()
}

#[tauri::command]
pub fn receita_install(app: AppHandle, id: String) -> Result<u32, String> {
    let program = program(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let emit = |event: TransactionEvent| {
            let _ = app.emit("software-transaction", event);
        };
        let report = |percent: u32, message: String| {
            emit(TransactionEvent::Progress {
                id: tx,
                percent,
                message,
            })
        };
        let result = install(program, &report);
        refresh_menu();
        emit(TransactionEvent::Finished {
            id: tx,
            success: result.is_ok(),
            message: result.err().unwrap_or_else(|| gettext("Concluído")),
        });
    });
    Ok(tx)
}

#[tauri::command]
pub fn receita_remove(app: AppHandle, id: String) -> Result<u32, String> {
    let program = program(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let result = remove(program);
        refresh_menu();
        let (success, message) = match result {
            Ok(Some(kept)) => (
                true,
                gettext("Removido. As declarações foram guardadas em {path}.")
                    .replace("{path}", &kept.display().to_string()),
            ),
            Ok(None) => (true, gettext("Concluído")),
            Err(error) => (false, error),
        };
        let _ = app.emit(
            "software-transaction",
            TransactionEvent::Finished {
                id: tx,
                success,
                message,
            },
        );
    });
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_are_pinned_to_the_receita_site() {
        for program in PROGRAMS {
            assert!(
                program
                    .url
                    .starts_with("https://downloadirpf.receita.fazenda.gov.br/")
            );
            assert!(program.url.ends_with(program.installer.file));
            assert_eq!(program.installer.sha256.len(), 64);
            assert!(
                program
                    .installer
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
            );
            assert!(program.installer.size > 0);
        }
    }

    #[test]
    fn installs_in_the_default_receita_folder() {
        let program = program("irpf2026").unwrap();
        let home = Path::new("/home/ana");
        assert_eq!(
            launcher(home, program),
            Path::new("/home/ana/ProgramasRFB/IRPF2026/IRPF2026")
        );
    }

    #[test]
    fn removal_keeps_the_declarations_and_only_installer_shortcuts_go() {
        let program = program("irpf2026").unwrap();
        let home = std::env::temp_dir().join(format!("vega-receita-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let dir = install_dir(&home, program);
        std::fs::create_dir_all(dir.join("aplicacao/dados/12345678900")).unwrap();
        // Remoção anterior já guardou declarações: a nova pasta ganha número.
        std::fs::create_dir_all(home.join("ProgramasRFB/IRPF2026-declarações")).unwrap();
        assert_eq!(
            declarations_target(&home, program),
            home.join("ProgramasRFB/IRPF2026-declarações (2)")
        );
        let apps = home.join("data/applications");
        std::fs::create_dir_all(&apps).unwrap();
        let ours = apps.join("install4j_x-IRPF2026.desktop");
        let other = apps.join("install4j_y-Outro.desktop");
        std::fs::write(&ours, format!("Exec=\"{}/IRPF2026\"\n", dir.display())).unwrap();
        std::fs::write(&other, "Exec=\"/opt/outro/Outro\"\n").unwrap();
        assert_eq!(installer_desktop_files(&home.join("data"), &dir), [ours]);
    }
}
