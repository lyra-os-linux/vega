//! Ferramentas Oracle na conta do usuário (vega#189).
//!
//! Instant Client (Basic, SQL*Plus e Tools), SQLcl, SQL Developer e Data
//! Modeler são publicados pela Oracle em download.oracle.com, sem login,
//! sob os Oracle Free Use Terms and Conditions. O Lyra não hospeda nada: o
//! Vega baixa a versão conferida pelo Lyra, confere tamanho e SHA-256 com a
//! conferência do download guiado (a Oracle não publica checksum nesses
//! links) e extrai em `~/.local/opt/oracle`, sem root. Uma versão nova só
//! entra depois que o Lyra confere o hash.
//!
//! Os comandos `sqlplus` e `sql` vão para `~/.local/bin` como scripts que
//! apontam para o Instant Client e para o Java 21 do Leap. Todos usam o mesmo
//! `TNS_ADMIN` (`~/.local/opt/oracle/network/admin`), cujo `tnsnames.ora`
//! nasce com o Oracle Database Free local (FREE e FREEPDB1, vega#190) e não é
//! sobrescrito depois.

use std::path::{Path, PathBuf};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::guided_download::{self, Expected};
use super::software::TransactionEvent;

const JAVA_HOME: &str = "/usr/lib64/jvm/java-21-openjdk-21";

#[derive(Debug, Clone, Copy)]
pub struct Artifact {
    pub url: &'static str,
    pub expected: Expected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    InstantClient,
    Sqlcl,
    /// Aplicativo Java com atalho: (pasta, script, .conf onde vai o JDK).
    Desktop {
        dir: &'static str,
        script: &'static str,
        conf: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    /// Arquivo que só existe depois de instalado, relativo a `~/.local/opt/oracle`.
    pub marker: &'static str,
    /// Pasta criada pela extração, apagada na remoção.
    pub dir: &'static str,
    pub kind: Kind,
    pub artifacts: &'static [Artifact],
}

const PAGE_IC: &str =
    "https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html";
const PAGE_SQLDEV: &str = "https://www.oracle.com/database/sqldeveloper/technologies/download/";

pub const TOOLS: &[Tool] = &[
    Tool {
        id: "oracle-instantclient",
        name: "Oracle Instant Client 23.26",
        marker: "instantclient_23_26/sqlplus",
        dir: "instantclient_23_26",
        kind: Kind::InstantClient,
        artifacts: &[
            Artifact {
                url: "https://download.oracle.com/otn_software/linux/instantclient/2326400/instantclient-basic-linux.x64-23.26.4.0.0.zip",
                expected: Expected {
                    page: PAGE_IC,
                    file: "instantclient-basic-linux.x64-23.26.4.0.0.zip",
                    size: 137_184_838,
                    sha256: "cab59aa3b411b3b50898e456fa9aabfe48d67e25cbc1bb23303800390ce7de26",
                    version: "23.26.4.0.0",
                },
            },
            Artifact {
                url: "https://download.oracle.com/otn_software/linux/instantclient/2326400/instantclient-sqlplus-linux.x64-23.26.4.0.0.zip",
                expected: Expected {
                    page: PAGE_IC,
                    file: "instantclient-sqlplus-linux.x64-23.26.4.0.0.zip",
                    size: 5_804_958,
                    sha256: "3cd43219cbaad59bb75cee18e7af074ada707446f13397b6f93ec7e4ca870490",
                    version: "23.26.4.0.0",
                },
            },
            Artifact {
                url: "https://download.oracle.com/otn_software/linux/instantclient/2326400/instantclient-tools-linux.x64-23.26.4.0.0.zip",
                expected: Expected {
                    page: PAGE_IC,
                    file: "instantclient-tools-linux.x64-23.26.4.0.0.zip",
                    size: 1_448_971,
                    sha256: "95e60dedcaf64a4ebc4dded73f5614c494a8c17fa272cd3c0485e0d13e825000",
                    version: "23.26.4.0.0",
                },
            },
        ],
    },
    Tool {
        id: "oracle-sqlcl",
        name: "Oracle SQLcl 26.3",
        marker: "sqlcl/bin/sql",
        dir: "sqlcl",
        kind: Kind::Sqlcl,
        artifacts: &[Artifact {
            url: "https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-26.3.1.301.1413.zip",
            expected: Expected {
                page: "https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/download/",
                file: "sqlcl-26.3.1.301.1413.zip",
                size: 124_053_650,
                sha256: "f3f188ae06f17eb181f77ae0be54f2ae8c165e52056a7cb7401a974cace9e5f2",
                version: "26.3.1",
            },
        }],
    },
    Tool {
        id: "oracle-sqldeveloper",
        name: "Oracle SQL Developer 26.2",
        marker: "sqldeveloper/sqldeveloper.sh",
        dir: "sqldeveloper",
        kind: Kind::Desktop {
            dir: "sqldeveloper",
            script: "sqldeveloper.sh",
            conf: "sqldeveloper/bin/sqldeveloper.conf",
        },
        artifacts: &[Artifact {
            url: "https://download.oracle.com/otn_software/java/sqldeveloper/sqldeveloper-26.2.0.186.2220-no-jre.zip",
            expected: Expected {
                page: PAGE_SQLDEV,
                file: "sqldeveloper-26.2.0.186.2220-no-jre.zip",
                size: 509_841_799,
                sha256: "6942c073b0b6df09a71503dfd754c6c7da7d5addc30a5e89bcee674f31fcdb7d",
                version: "26.2.0",
            },
        }],
    },
    Tool {
        id: "oracle-datamodeler",
        name: "Oracle SQL Developer Data Modeler 24.3",
        marker: "datamodeler/datamodeler.sh",
        dir: "datamodeler",
        kind: Kind::Desktop {
            dir: "datamodeler",
            script: "datamodeler.sh",
            conf: "datamodeler/bin/datamodeler.conf",
        },
        artifacts: &[Artifact {
            url: "https://download.oracle.com/otn_software/java/sqldeveloper/datamodeler-24.3.1.351.0831-no-jre.zip",
            expected: Expected {
                page: "https://www.oracle.com/database/sqldeveloper/technologies/sql-data-modeler/download/",
                file: "datamodeler-24.3.1.351.0831-no-jre.zip",
                size: 384_389_525,
                sha256: "3c6e01189fbd7f0faf66955a1afc92447f4fb20453b9ff491f4c014f83b39af5",
                version: "24.3.1",
            },
        }],
    },
];

pub fn tool(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.id == id)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn data_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"))
}

fn cache_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".cache"))
        .join("vega")
}

pub fn opt_dir(home: &Path) -> PathBuf {
    home.join(".local/opt/oracle")
}

pub fn tns_admin(home: &Path) -> PathBuf {
    opt_dir(home).join("network/admin")
}

/// Conexões com o Oracle Database Free local (vega#190).
pub const TNSNAMES: &str = "# Gerado pelo Vega (vega#189). Pode editar: o Vega não sobrescreve.\n\
FREE =\n  (DESCRIPTION =\n    (ADDRESS = (PROTOCOL = TCP)(HOST = localhost)(PORT = 1521))\n    (CONNECT_DATA = (SERVICE_NAME = FREE)))\n\n\
FREEPDB1 =\n  (DESCRIPTION =\n    (ADDRESS = (PROTOCOL = TCP)(HOST = localhost)(PORT = 1521))\n    (CONNECT_DATA = (SERVICE_NAME = FREEPDB1)))\n";

/// Comandos de `~/.local/bin` criados por cada ferramenta.
fn commands(tool: &Tool) -> &'static [&'static str] {
    match tool.kind {
        Kind::InstantClient => &["sqlplus"],
        Kind::Sqlcl => &["sql"],
        Kind::Desktop { .. } => &[],
    }
}

/// Script de `~/.local/bin/<comando>`.
pub fn wrapper(tool: &Tool, home: &Path) -> String {
    let opt = opt_dir(home);
    let tns = tns_admin(home);
    match tool.kind {
        Kind::InstantClient => format!(
            "#!/bin/sh\n# Gerado pelo Vega (vega#189).\nIC=\"{ic}\"\nexport LD_LIBRARY_PATH=\"$IC${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}\"\nexport TNS_ADMIN=\"${{TNS_ADMIN:-{tns}}}\"\nexec \"$IC/sqlplus\" \"$@\"\n",
            ic = opt.join(tool.dir).display(),
            tns = tns.display(),
        ),
        Kind::Sqlcl => format!(
            "#!/bin/sh\n# Gerado pelo Vega (vega#189).\nexport JAVA_HOME=\"${{JAVA_HOME:-{JAVA_HOME}}}\"\nexport TNS_ADMIN=\"${{TNS_ADMIN:-{tns}}}\"\nexec \"{sql}\" \"$@\"\n",
            tns = tns.display(),
            sql = opt.join("sqlcl/bin/sql").display(),
        ),
        Kind::Desktop { .. } => String::new(),
    }
}

fn desktop_path(home: &Path, tool: &Tool) -> PathBuf {
    data_dir(home)
        .join("applications")
        .join(format!("lyra-{}.desktop", tool.id))
}

pub fn desktop_entry(tool: &Tool, home: &Path) -> String {
    let Kind::Desktop { dir, script, .. } = tool.kind else {
        return String::new();
    };
    let base = opt_dir(home).join(dir);
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=env TNS_ADMIN=\"{tns}\" sh \"{script}\"\nPath={base}\nIcon=database\nTerminal=false\nCategories=Development;Database;\nX-Lyra-Vega=oracle\n",
        name = tool.name,
        tns = tns_admin(home).display(),
        script = base.join(script).display(),
        base = base.display(),
    )
}

fn write_file(path: &Path, text: &str, mode: u32) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let _ = std::fs::remove_file(path);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

fn unzip(zip: &Path, target: &Path) -> Result<(), String> {
    let mut command = std::process::Command::new("unzip");
    command.arg("-q").arg("-o").arg(zip).arg("-d").arg(target);
    crate::webkit_env::clean_command(&mut command);
    let status = command.status().map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(gettext("Não foi possível extrair {file} ({status}).")
            .replace("{file}", &zip.display().to_string())
            .replace("{status}", &status.to_string()))
    }
}

fn install(tool: &Tool, report: &dyn Fn(u32, String)) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let opt = opt_dir(&home);
    let cache = cache_dir(&home);
    std::fs::create_dir_all(&opt).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let total = tool.artifacts.len() as u32;
    for (index, artifact) in tool.artifacts.iter().enumerate() {
        let step = 80 * index as u32 / total;
        let zip = cache.join(artifact.expected.file);
        report(
            5 + step,
            gettext("Baixando {file} do site da Oracle…").replace("{file}", artifact.expected.file),
        );
        let result = super::receita_apps::download(artifact.url, &zip)
            .and_then(|()| {
                guided_download::verify(&artifact.expected, &zip)
                    .map_err(|rejection| rejection.message(&artifact.expected))
            })
            .and_then(|()| unzip(&zip, &opt));
        let _ = std::fs::remove_file(&zip);
        result?;
    }
    report(90, gettext("Configurando…"));
    let tnsnames = tns_admin(&home).join("tnsnames.ora");
    if !tnsnames.is_file() {
        write_file(&tnsnames, TNSNAMES, 0o644)?;
    }
    for command in commands(tool) {
        write_file(
            &home.join(".local/bin").join(command),
            &wrapper(tool, &home),
            0o755,
        )?;
    }
    if let Kind::Desktop { dir, conf, .. } = tool.kind {
        // Versões "no-jre": o JDK 21 do Leap vai no .conf para o app não
        // perguntar o caminho no primeiro início.
        let conf = opt.join(dir).join(conf);
        let text = std::fs::read_to_string(&conf).map_err(|error| error.to_string())?;
        if !text.contains("SetJavaHome") {
            std::fs::write(&conf, format!("{text}\nSetJavaHome {JAVA_HOME}\n"))
                .map_err(|error| error.to_string())?;
        }
        write_file(
            &desktop_path(&home, tool),
            &desktop_entry(tool, &home),
            0o644,
        )?;
    }
    if !opt.join(tool.marker).is_file() {
        return Err(gettext("A instalação terminou sem o arquivo esperado."));
    }
    Ok(())
}

fn remove(tool: &Tool) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    for command in commands(tool) {
        let _ = std::fs::remove_file(home.join(".local/bin").join(command));
    }
    let _ = std::fs::remove_file(desktop_path(&home, tool));
    let dir = opt_dir(&home).join(tool.dir);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn refresh_menu(home: &Path) {
    let mut command = std::process::Command::new("update-desktop-database");
    command.arg(data_dir(home).join("applications"));
    crate::webkit_env::clean_command(&mut command);
    let _ = command.status();
}

#[tauri::command]
pub fn oracle_installed() -> Vec<String> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let opt = opt_dir(&home);
    TOOLS
        .iter()
        .filter(|tool| opt.join(tool.marker).is_file())
        .map(|tool| tool.id.to_owned())
        .collect()
}

fn spawn(app: AppHandle, tool: &'static Tool, install_it: bool) -> u32 {
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let report = |percent: u32, message: String| {
            let _ = app.emit(
                "software-transaction",
                TransactionEvent::Progress {
                    id: tx,
                    percent,
                    message,
                },
            );
        };
        let result = if install_it {
            install(tool, &report)
        } else {
            remove(tool)
        };
        if let Some(home) = home() {
            refresh_menu(&home);
        }
        let _ = app.emit(
            "software-transaction",
            TransactionEvent::Finished {
                id: tx,
                success: result.is_ok(),
                message: result.err().unwrap_or_else(|| gettext("Concluído")),
            },
        );
    });
    tx
}

#[tauri::command]
pub fn oracle_install(app: AppHandle, id: String) -> Result<u32, String> {
    let tool = tool(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    Ok(spawn(app, tool, true))
}

#[tauri::command]
pub fn oracle_remove(app: AppHandle, id: String) -> Result<u32, String> {
    let tool = tool(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    Ok(spawn(app, tool, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifacts_are_pinned_to_oracle_downloads() {
        for tool in TOOLS {
            for artifact in tool.artifacts {
                assert!(
                    artifact
                        .url
                        .starts_with("https://download.oracle.com/otn_software/"),
                    "{}",
                    tool.id
                );
                assert!(
                    artifact.url.ends_with(artifact.expected.file),
                    "{}",
                    tool.id
                );
                assert!(!artifact.url.contains("latest"), "{}", tool.id);
                assert_eq!(artifact.expected.sha256.len(), 64, "{}", tool.id);
            }
        }
    }

    #[test]
    fn wrappers_point_to_the_user_install() {
        let home = Path::new("/home/u");
        let sqlplus = wrapper(tool("oracle-instantclient").unwrap(), home);
        assert!(sqlplus.contains("IC=\"/home/u/.local/opt/oracle/instantclient_23_26\""));
        assert!(sqlplus.contains("TNS_ADMIN:-/home/u/.local/opt/oracle/network/admin"));
        assert!(sqlplus.contains("exec \"$IC/sqlplus\" \"$@\""));
        let sql = wrapper(tool("oracle-sqlcl").unwrap(), home);
        assert!(sql.contains("JAVA_HOME:-/usr/lib64/jvm/java-21-openjdk-21"));
        assert!(sql.contains("exec \"/home/u/.local/opt/oracle/sqlcl/bin/sql\""));
    }

    #[test]
    fn desktop_entry_runs_the_script_with_tns_admin() {
        let entry = desktop_entry(tool("oracle-sqldeveloper").unwrap(), Path::new("/home/u"));
        assert!(entry.contains(
            "Exec=env TNS_ADMIN=\"/home/u/.local/opt/oracle/network/admin\" sh \"/home/u/.local/opt/oracle/sqldeveloper/sqldeveloper.sh\""
        ));
    }

    #[test]
    fn tnsnames_has_the_local_free_database() {
        assert!(TNSNAMES.contains("FREEPDB1 ="));
        assert!(TNSNAMES.contains("(PORT = 1521)"));
    }
}
