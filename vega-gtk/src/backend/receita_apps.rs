//! Programas da Receita Federal na conta do usuário (vega#212).
//!
//! A Receita publica o IRPF para Linux sem pedir login, como instalador
//! install4j que já traz o próprio Java. O Vega baixa a versão conferida pelo
//! Lyra, confere tamanho e SHA-256 (a Receita não publica checksum) com a
//! conferência do download guiado e roda o instalador em modo silencioso, sem
//! root, em `~/ProgramasRFB/<programa>`, a pasta padrão da Receita. Uma
//! versão nova da Receita só é instalada depois que o Lyra confere o hash.
//!
//! O Vega não remove o programa: as declarações ficam dentro da pasta dele, e
//! apagá-la por engano faria o contribuinte perder o trabalho.

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
    pub comment: &'static str,
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
        comment: "Declaração do Imposto de Renda Pessoa Física 2026",
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
        comment: "Declaração do Imposto de Renda Pessoa Física 2025",
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

fn desktop_path(data: &Path, program: &Program) -> PathBuf {
    data.join(format!("applications/lyra-receita-{}.desktop", program.id))
}

pub fn desktop_entry(program: &Program, launcher: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\nExec=\"{launcher}\"\nPath={dir}\nIcon={icon}\nTerminal=false\nCategories=Office;Finance;\n",
        name = program.name,
        comment = program.comment,
        launcher = launcher.display(),
        dir = launcher.parent().unwrap_or(launcher).display(),
        icon = icon.display(),
    )
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
    result?;
    let target = launcher(&home, program);
    let icon = install_dir(&home, program)
        .join(".install4j")
        .join(format!("{}.png", program.dir));
    let entry = desktop_path(&data, program);
    std::fs::create_dir_all(entry.parent().unwrap_or(&data)).map_err(|error| error.to_string())?;
    std::fs::write(&entry, desktop_entry(program, &target, &icon))
        .map_err(|error| error.to_string())
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
        let entry = desktop_entry(
            program,
            &launcher(home, program),
            Path::new("/home/ana/ProgramasRFB/IRPF2026/.install4j/IRPF2026.png"),
        );
        assert!(entry.contains("Exec=\"/home/ana/ProgramasRFB/IRPF2026/IRPF2026\"\n"));
        assert!(entry.contains("Path=/home/ana/ProgramasRFB/IRPF2026\n"));
    }
}
