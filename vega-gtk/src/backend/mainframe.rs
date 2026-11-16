//! Mainframe de treinamento: MVS 3.8j TK5 no Hercules (vega#194).
//!
//! O TK5 (Rob Prins) é o "turnkey" atual do MVS 3.8j. O status de domínio
//! público do MVS 3.8j é contestado, então o Lyra não hospeda a imagem: o
//! Vega baixa o `mvs-tk5.zip` do site do autor, depois do aviso de licença na
//! interface, e só aceita o arquivo com o tamanho e o SHA-256 conferidos pelo
//! Lyra (o site não publica checksum). Tudo fica na conta do usuário, sem
//! root, em `~/.local/share/lyra-mainframe/tk5`.
//!
//! O zip traz binários próprios do Hercules; eles não são extraídos. O TK5
//! roda no `hercules` dos repositórios do Lyra, com a mesma configuração do
//! script `mvs_ipl` do TK5. As portas ficam só em 127.0.0.1: terminais 3270
//! (`CNSLPORT`) e leitor de cartões (`RDRPORT`). O painel HTTP do Hercules não
//! aceita endereço; ele exige uma senha aleatória, guardada só na conta do
//! usuário, e serve apenas para o atalho de desligar mandar o
//! `scripts/shutdown` do TK5 (desliga o MVS antes de parar o Hercules, para
//! não corromper os volumes).
//!
//! O Hercules roda numa unidade transitória do systemd do usuário
//! (`lyra-mainframe`): fechar uma janela não derruba o MVS. Reinstalar não
//! apaga os volumes já extraídos.

use std::io::Write;
use std::path::{Path, PathBuf};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::guided_download::{self, Expected};
use super::software::TransactionEvent;

pub const ID: &str = "tk5";

const URL: &str = "https://www.prince-webdesign.nl/images/downloads/mvs-tk5.zip";

pub const TK5: Expected = Expected {
    page: "https://www.prince-webdesign.nl/index.php/software/mvs-3-8j-turnkey-5",
    file: "mvs-tk5.zip",
    size: 498_312_872,
    sha256: "710d002843631322810a276dd42c793fda458548dc64d86e2914a62db7425f84",
    version: "Update 5",
};

/// Binários do Hercules que vêm no zip e não são usados.
const SKIP: &[&str] = &[
    "mvs-tk5/hercules/linux/*",
    "mvs-tk5/hercules/windows/*",
    "mvs-tk5/hercules/darwin/*",
    "mvs-tk5/hercules/external_packages_binaries/*",
];

const LAUNCHER: &str = r#"#!/bin/bash
# Gerado pelo Vega (vega#194): liga, desliga e conecta ao MVS 3.8j TK5.
set -u
BASE="${XDG_DATA_HOME:-$HOME/.local/share}/lyra-mainframe"
DIR="$BASE/tk5"
UNIT=lyra-mainframe
running() { systemctl --user is-active --quiet "$UNIT"; }
say() {
  echo "$1"
  gdbus call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications \
    --method org.freedesktop.Notifications.Notify "@NAME@" 0 computer "@NAME@" "$1" '[]' '{}' -1 >/dev/null 2>&1 || true
}
listening() { (exec 3<>/dev/tcp/127.0.0.1/3270) 2>/dev/null; }
start() {
  running && return 0
  systemd-run --user --quiet --collect --unit="$UNIT" --working-directory="$DIR" \
    --setenv=HERCULES_RC=scripts/ipl.rc \
    --setenv=CNSLPORT=127.0.0.1:3270 \
    --setenv=RDRPORT=127.0.0.1:3505 \
    --setenv="HTTPPORT=8038 AUTH lyra $(cat "$BASE/http-secret")" \
    /bin/bash -c 'exec /usr/bin/hercules -d -f conf/tk5.cnf < <(exec sleep infinity)' || exit 1
  say "@STARTING@"
}
wait_port() {
  for _ in $(seq 120); do listening && return 0; sleep 1; done
  say "@NOPORT@"; exit 1
}
case "${1:-}" in
  start) start ;;
  stop)
    running || { say "@OFF@"; exit 0; }
    curl -fsS -u "lyra:$(cat "$BASE/http-secret")" \
      "http://127.0.0.1:8038/cgi-bin/tasks/cmd?cmd=script%20scripts/shutdown" >/dev/null \
      && say "@STOPPING@" || { say "@STOPFAIL@"; exit 1; }
    ;;
  tso) start; wait_port; exec x3270 -model 3279-4 127.0.0.1:3270 ;;
  console) start; wait_port; exec c3270 -model 3279-4 127.0.0.1:3270 ;;
  *) echo "uso: $0 start|stop|tso|console" >&2; exit 2 ;;
esac
"#;

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

pub fn base_dir(data: &Path) -> PathBuf {
    data.join("lyra-mainframe")
}

fn launcher(data: &Path) -> PathBuf {
    base_dir(data).join("lyra-mainframe")
}

/// Atalhos do menu: (arquivo, nome, ação do lançador, ícone, no terminal).
fn shortcuts() -> [(&'static str, String, &'static str, &'static str, bool); 4] {
    [
        (
            "lyra-mainframe-start",
            gettext("Ligar mainframe (MVS 3.8j)"),
            "start",
            "computer",
            false,
        ),
        (
            "lyra-mainframe-tso",
            gettext("Terminal TSO (x3270)"),
            "tso",
            "utilities-terminal",
            false,
        ),
        (
            "lyra-mainframe-console",
            gettext("Terminal TSO no console (c3270)"),
            "console",
            "utilities-terminal",
            true,
        ),
        (
            "lyra-mainframe-stop",
            gettext("Desligar mainframe (MVS 3.8j)"),
            "stop",
            "system-shutdown",
            false,
        ),
    ]
}

fn desktop_path(data: &Path, file: &str) -> PathBuf {
    data.join("applications").join(format!("{file}.desktop"))
}

pub fn desktop_entry(name: &str, exec: &Path, action: &str, icon: &str, terminal: bool) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{}\" {action}\nIcon={icon}\nTerminal={terminal}\nCategories=Development;Emulator;\nX-Lyra-Vega=mainframe\n",
        exec.display()
    )
}

/// Texto seguro dentro de aspas duplas do bash.
pub fn shell_text(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '"' | '$' | '`' | '\\' | '\n'))
        .collect()
}

/// Lançador com os textos já traduzidos (o script não tem gettext).
pub fn launcher_script() -> String {
    let t = |text: &str| shell_text(&gettext(text));
    LAUNCHER
        .replace("@NAME@", &t("Mainframe de treinamento"))
        .replace(
            "@STARTING@",
            &t("Ligando o MVS. O logon no TSO fica disponível em um ou dois minutos."),
        )
        .replace(
            "@NOPORT@",
            &t("O mainframe não abriu a porta 3270. Veja o log com: journalctl --user -u lyra-mainframe"),
        )
        .replace("@OFF@", &t("O mainframe já está desligado."))
        .replace(
            "@STOPPING@",
            &t("Desligando o MVS. Saia do TSO; o Hercules fecha sozinho em alguns minutos."),
        )
        .replace(
            "@STOPFAIL@",
            &t("Não foi possível pedir o desligamento ao Hercules."),
        )
}

fn random_secret() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut bytes))
        .map_err(|error| error.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn write_private(path: &Path, text: &str, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
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

/// Argumentos do `unzip`: extrai em `target` sem os binários do Hercules.
pub fn unzip_arguments(zip: &Path, target: &Path) -> Vec<String> {
    let mut args = vec![
        "-q".to_owned(),
        "-o".to_owned(),
        zip.display().to_string(),
        "-d".to_owned(),
        target.display().to_string(),
        "-x".to_owned(),
    ];
    args.extend(SKIP.iter().map(|pattern| (*pattern).to_owned()));
    args
}

fn extract(zip: &Path, base: &Path) -> Result<(), String> {
    let staging = base.join(".tk5-extraindo");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let mut command = std::process::Command::new("unzip");
    command.args(unzip_arguments(zip, &staging));
    crate::webkit_env::clean_command(&mut command);
    let status = command.status().map_err(|error| error.to_string())?;
    let extracted = staging.join("mvs-tk5");
    if !status.success() || !extracted.join("conf/tk5.cnf").is_file() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(gettext("Não foi possível extrair o TK5 ({status}).")
            .replace("{status}", &status.to_string()));
    }
    std::fs::rename(&extracted, base.join("tk5")).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

fn install(report: &dyn Fn(u32, String)) -> Result<(), String> {
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let cache = cache_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let base = base_dir(&data);
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    // Reinstalar mantém os volumes (DASD) com o trabalho do usuário.
    if !base.join("tk5/conf/tk5.cnf").is_file() {
        std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let zip = cache.join(TK5.file);
        report(5, gettext("Baixando o TK5 (475 MB) do site de Rob Prins…"));
        let result = super::receita_apps::download(URL, &zip)
            .and_then(|()| {
                report(60, gettext("Conferindo o arquivo…"));
                guided_download::verify(&TK5, &zip).map_err(|rejection| rejection.message(&TK5))
            })
            .and_then(|()| {
                report(70, gettext("Extraindo os volumes do MVS…"));
                extract(&zip, &base)
            });
        let _ = std::fs::remove_file(&zip);
        result?;
    }
    report(95, gettext("Criando os atalhos…"));
    let secret = base.join("http-secret");
    if !secret.is_file() {
        write_private(&secret, &random_secret()?, 0o600)?;
    }
    let exec = launcher(&data);
    write_private(&exec, &launcher_script(), 0o755)?;
    std::fs::create_dir_all(data.join("applications")).map_err(|error| error.to_string())?;
    for (file, name, action, icon, terminal) in shortcuts() {
        write_private(
            &desktop_path(&data, file),
            &desktop_entry(&name, &exec, action, icon, terminal),
            0o644,
        )?;
    }
    Ok(())
}

fn running() -> bool {
    let mut command = std::process::Command::new("systemctl");
    command.args(["--user", "is-active", "--quiet", "lyra-mainframe"]);
    crate::webkit_env::clean_command(&mut command);
    command.status().is_ok_and(|status| status.success())
}

fn remove() -> Result<(), String> {
    if running() {
        return Err(gettext(
            "O mainframe está ligado. Desligue-o pelo atalho \"Desligar mainframe\" e espere o Hercules fechar antes de remover.",
        ));
    }
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    for (file, ..) in shortcuts() {
        let _ = std::fs::remove_file(desktop_path(&data, file));
    }
    let base = base_dir(&data);
    if base.exists() {
        std::fs::remove_dir_all(&base).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn refresh_menu() {
    if let Some(data) = data_dir() {
        let mut command = std::process::Command::new("update-desktop-database");
        command.arg(data.join("applications"));
        crate::webkit_env::clean_command(&mut command);
        let _ = command.status();
    }
}

#[tauri::command]
pub fn mainframe_installed() -> Vec<String> {
    data_dir()
        .filter(|data| {
            launcher(data).is_file() && base_dir(data).join("tk5/conf/tk5.cnf").is_file()
        })
        .map(|_| vec![ID.to_owned()])
        .unwrap_or_default()
}

fn finish(app: &AppHandle, tx: u32, result: Result<String, String>) {
    refresh_menu();
    let _ = app.emit(
        "software-transaction",
        TransactionEvent::Finished {
            id: tx,
            success: result.is_ok(),
            message: result.unwrap_or_else(|error| error),
        },
    );
}

#[tauri::command]
pub fn mainframe_install(app: AppHandle, id: String) -> Result<u32, String> {
    if id != ID {
        return Err(gettext("Aplicativo desconhecido."));
    }
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
        let result = install(&report).map(|()| {
            gettext("Pronto. No menu de aplicativos, use \"Ligar mainframe\" e depois \"Terminal TSO (x3270)\". Logon HERC01, senha CUL8TR. Para desligar, use \"Desligar mainframe\".")
        });
        finish(&app, tx, result);
    });
    Ok(tx)
}

#[tauri::command]
pub fn mainframe_remove(app: AppHandle, id: String) -> Result<u32, String> {
    if id != ID {
        return Err(gettext("Aplicativo desconhecido."));
    }
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let result = remove().map(|()| gettext("Concluído"));
        finish(&app, tx, result);
    });
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tk5_is_pinned_to_the_author_site() {
        assert!(URL.starts_with("https://www.prince-webdesign.nl/"));
        assert!(URL.ends_with(TK5.file));
        assert_eq!(TK5.sha256.len(), 64);
        assert!(TK5.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn unzip_skips_the_bundled_hercules() {
        let args = unzip_arguments(Path::new("/c/mvs-tk5.zip"), Path::new("/d/x"));
        assert_eq!(
            &args[..6],
            ["-q", "-o", "/c/mvs-tk5.zip", "-d", "/d/x", "-x"]
        );
        assert!(args.contains(&"mvs-tk5/hercules/linux/*".to_owned()));
        assert!(!args.iter().any(|arg| arg.contains("httproot")));
    }

    #[test]
    fn launcher_binds_ports_to_localhost_and_needs_no_placeholders() {
        let script = launcher_script();
        assert!(!script.contains('@'));
        assert!(script.contains("CNSLPORT=127.0.0.1:3270"));
        assert!(script.contains("RDRPORT=127.0.0.1:3505"));
        assert!(script.contains("HTTPPORT=8038 AUTH lyra"));
        assert!(script.contains("scripts/shutdown"));
    }

    #[test]
    fn shell_text_drops_quotes_and_expansions() {
        assert_eq!(shell_text("a \"b\" $(x) `y` \\z"), "a b (x) y z");
    }

    #[test]
    fn desktop_entry_quotes_the_launcher() {
        let entry = desktop_entry(
            "Ligar",
            Path::new("/home/u/.local/share/lyra-mainframe/lyra-mainframe"),
            "start",
            "computer",
            false,
        );
        assert!(
            entry.contains("Exec=\"/home/u/.local/share/lyra-mainframe/lyra-mainframe\" start\n")
        );
        assert!(entry.contains("Terminal=false\n"));
    }
}
