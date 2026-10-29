//! Atualização de um app Flatpak pela própria sessão do usuário.
//!
//! O Flatpak libera atualizar (e, para quem é do grupo wheel, instalar e
//! remover) sem senha a quem está numa sessão local ativa: o polkit dele olha
//! a sessão do processo que chama. Pelo vegad, o comando nascia fora da
//! sessão e o Vega ainda exigia a ação administrativa própria, então toda
//! atualização de Flatpak pedia senha. Aqui o Vega roda `flatpak update` como
//! o usuário, e o Flatpak (com o controle parental do malcontent) decide.
//! Os eventos são os mesmos `software-transaction` das transações do vegad.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::software::TransactionEvent;

/// Ids locais ficam acima dos do vegad, para as tarefas não colidirem.
static NEXT_ID: AtomicU32 = AtomicU32::new(0x8000_0000);

/// Mesmo critério do vegad (`validateFlatpakAppID`): nome reverso-DNS, sem
/// nada que vire opção ou caminho.
pub fn valid_app_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= 3
        && !id.starts_with('-')
        && id.len() <= 255
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                && !part.starts_with(|c: char| c.is_ascii_digit())
        })
}

/// Instalação onde o app está: "user" se estiver só na do usuário.
pub fn installation_of(list: &str, id: &str) -> &'static str {
    let mut user = false;
    for line in list.lines() {
        let mut fields = line.split('\t');
        if fields.next() == Some(id) {
            match fields.next().map(str::trim) {
                Some("system") => return "system",
                Some("user") => user = true,
                _ => {}
            }
        }
    }
    if user { "user" } else { "system" }
}

fn flatpak(args: &[&str]) -> Command {
    let mut command = Command::new("flatpak");
    command.args(args);
    crate::webkit_env::clean_command(&mut command);
    command
}

/// Começa a atualização e devolve o id da tarefa na hora.
pub fn start(app: AppHandle, id: String) -> Result<u32, String> {
    if !valid_app_id(&id) {
        return Err(gettext("Identificador de aplicativo Flatpak inválido."));
    }
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || run(app, tx, id));
    Ok(tx)
}

fn run(app: AppHandle, tx: u32, id: String) {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    let list = flatpak(&["list", "--app", "--columns=application,installation"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    let scope = format!("--{}", installation_of(&list, &id));
    emit(TransactionEvent::Progress {
        id: tx,
        percent: 0,
        message: gettext("Atualizando {app}…").replace("{app}", &id),
    });
    let child = flatpak(&[
        "update",
        "-y",
        "--noninteractive",
        "--app",
        &scope,
        "--",
        &id,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn();
    let mut child = match child {
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
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim().to_owned();
            if line.is_empty() {
                continue;
            }
            // "flatpak", não "stdout": a interface ignora o stdout do vegad,
            // que repete os eventos de pacote.
            emit(TransactionEvent::Console {
                id: tx,
                source: "flatpak".into(),
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
    let success = child.wait().map(|status| status.success()).unwrap_or(false);
    emit(TransactionEvent::Finished {
        id: tx,
        success,
        message: if success {
            gettext("Concluído")
        } else if last_error.is_empty() {
            gettext("A atualização do Flatpak falhou.")
        } else {
            last_error
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_ids_are_validated_like_vegad() {
        for good in [
            "org.jousse.vincent.Pomodorolm",
            "us.zoom.Zoom",
            "com.obsproject.Studio",
            "org.gnome.Platform-x",
        ] {
            assert!(valid_app_id(good), "{good}");
        }
        for bad in [
            "--system",
            "zoom",
            "org..Zoom",
            "org.gnome/../x",
            "-org.a.b",
            "org.1x.App",
            "org.a.b c",
        ] {
            assert!(!valid_app_id(bad), "{bad}");
        }
    }

    #[test]
    fn installation_prefers_system_and_falls_back_to_user() {
        let list = "us.zoom.Zoom\tsystem\norg.example.Only\tuser\norg.example.Both\tuser\norg.example.Both\tsystem\n";
        assert_eq!(installation_of(list, "us.zoom.Zoom"), "system");
        assert_eq!(installation_of(list, "org.example.Only"), "user");
        assert_eq!(installation_of(list, "org.example.Both"), "system");
        assert_eq!(installation_of(list, "org.example.Absent"), "system");
    }
}
