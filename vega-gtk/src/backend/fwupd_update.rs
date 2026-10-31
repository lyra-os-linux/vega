//! Atualização de firmware pela própria sessão do usuário
//! (add-diagnostics-extras). O fwupd libera atualizar firmware assinado de
//! dispositivos internos confiáveis para a sessão local ativa
//! (`org.freedesktop.fwupd.update-internal-trusted`); o polkit dele decide se
//! pede senha, não a ação administrativa do Vega. Mesmo padrão do
//! `flatpak_session`: eventos `software-transaction`, tarefa e console.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::software::TransactionEvent;

/// Ids locais acima dos do vegad e dos do Flatpak.
static NEXT_ID: AtomicU32 = AtomicU32::new(0xC000_0000);

/// O fwupd termina com 2 quando não há nada para atualizar.
const NOTHING_TO_DO: i32 = 2;

/// Linha do fwupd que pede reinício para concluir a instalação.
pub fn asks_for_reboot(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    line.contains("reboot") || line.contains("restart")
}

pub fn start(app: AppHandle) -> Result<u32, String> {
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || run(app, tx));
    Ok(tx)
}

fn run(app: AppHandle, tx: u32) {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    emit(TransactionEvent::Progress {
        id: tx,
        percent: 0,
        message: gettext("Atualizando o firmware…"),
    });
    let mut command = Command::new("fwupdmgr");
    command
        .args(["update", "-y", "--no-reboot-check"])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::webkit_env::clean_command(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            emit(TransactionEvent::Finished {
                id: tx,
                success: false,
                message: error.to_string(),
            });
            return;
        }
    };
    let stderr = child.stderr.take();
    let err_app = app.clone();
    let errors = std::thread::spawn(move || {
        let mut last = String::new();
        if let Some(stderr) = stderr {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }
                last = line.clone();
                let _ = err_app.emit(
                    "software-transaction",
                    TransactionEvent::Console {
                        id: tx,
                        source: "stderr".into(),
                        line,
                    },
                );
            }
        }
        last
    });
    let mut reboot = false;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim().to_owned();
            if line.is_empty() {
                continue;
            }
            reboot |= asks_for_reboot(&line);
            emit(TransactionEvent::Console {
                id: tx,
                source: "fwupd".into(),
                line: line.clone(),
            });
            emit(TransactionEvent::Progress {
                id: tx,
                percent: 50,
                message: line,
            });
        }
    }
    let last_error = errors.join().unwrap_or_default();
    reboot |= asks_for_reboot(&last_error);
    let code = child.wait().ok().and_then(|status| status.code());
    let (success, message) = match code {
        Some(0) if reboot => (
            true,
            gettext("Firmware atualizado. Reinicie para concluir."),
        ),
        Some(0) => (true, gettext("Firmware atualizado.")),
        Some(NOTHING_TO_DO) => (true, gettext("Nenhuma atualização de firmware pendente.")),
        _ if !last_error.is_empty() => (false, last_error),
        _ => (false, gettext("A atualização de firmware falhou.")),
    };
    emit(TransactionEvent::Finished {
        id: tx,
        success,
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reboot_requests_are_recognized() {
        assert!(asks_for_reboot("An update requires a reboot to complete."));
        assert!(asks_for_reboot("Restart the system to complete the update"));
        assert!(!asks_for_reboot("Downloading… 45%"));
    }
}
