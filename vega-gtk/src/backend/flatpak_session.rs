//! Instalação, atualização e remoção de apps Flatpak pela própria sessão do
//! usuário.
//!
//! O Flatpak libera essas operações sem senha a quem é do grupo wheel numa
//! sessão local ativa (60-org.freedesktop.Flatpak.rules): o polkit dele olha
//! a sessão do processo que chama. Pelo vegad, o comando nascia fora da
//! sessão e o Vega ainda exigia a ação administrativa própria, então toda
//! operação de Flatpak pedia senha. Aqui o Vega roda o `flatpak` como o
//! usuário, e o Flatpak (com o controle parental do malcontent) decide; quem
//! não é do wheel recebe o pedido de senha do próprio Flatpak. Isso também
//! tira do vegad o script apply_extra dos apps com extra-data (Zoom,
//! Spotify...), que roda no flatpak-system-helper.
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

/// Operação Flatpak feita na sessão do usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Install,
    Update,
    Remove,
}

impl Operation {
    fn starting(self) -> String {
        match self {
            Operation::Install => gettext("Instalando {app}…"),
            Operation::Update => gettext("Atualizando {app}…"),
            Operation::Remove => gettext("Removendo {app}…"),
        }
    }

    fn failed(self) -> String {
        match self {
            Operation::Install => gettext("A instalação do Flatpak falhou."),
            Operation::Update => gettext("A atualização do Flatpak falhou."),
            Operation::Remove => gettext("A remoção do Flatpak falhou."),
        }
    }
}

/// Argumentos do `flatpak`. Instalar vai sempre para o sistema, a partir do
/// Flathub, como o vegad fazia; atualizar e remover usam a instalação onde o
/// app está (`scope`).
pub fn arguments(operation: Operation, id: &str, scope: &str) -> Vec<String> {
    let mut args: Vec<String> = match operation {
        Operation::Install => vec!["install".into()],
        Operation::Update => vec!["update".into()],
        Operation::Remove => vec!["uninstall".into()],
    };
    args.extend(["-y", "--noninteractive", "--app"].map(String::from));
    match operation {
        Operation::Install => args.extend(["--system", "--", "flathub"].map(String::from)),
        _ => args.extend([format!("--{scope}"), "--".into()]),
    }
    args.push(id.to_owned());
    args
}

/// Começa a operação e devolve o id da tarefa na hora.
pub fn start(app: AppHandle, operation: Operation, id: String) -> Result<u32, String> {
    if !valid_app_id(&id) {
        return Err(gettext("Identificador de aplicativo Flatpak inválido."));
    }
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || run(app, tx, operation, id));
    Ok(tx)
}

fn run(app: AppHandle, tx: u32, operation: Operation, id: String) {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    let scope = if operation == Operation::Install {
        "system"
    } else {
        let list = flatpak(&["list", "--app", "--columns=application,installation"])
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        installation_of(&list, &id)
    };
    emit(TransactionEvent::Progress {
        id: tx,
        percent: 0,
        message: operation.starting().replace("{app}", &id),
    });
    let args = arguments(operation, &id, scope);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    follow(&app, tx, flatpak(&args), "flatpak", operation.failed());
}

/// Roda `command` e repassa a saída como eventos `software-transaction` da
/// tarefa `tx`: stdout vira console (com `source`) e avança o progresso a
/// cada linha; stderr vira console e a última linha é a mensagem de erro.
/// Devolve se o comando terminou com sucesso.
///
/// O fim da tarefa é o fim do processo, não o fim da saída: o Wine deixa
/// processos de fundo (wineserver, services.exe, o app recém-instalado) que
/// herdam os pipes e os mantêm abertos depois que o winetricks sai. As
/// leituras correm em threads próprias e, depois do `wait`, recebem só um
/// instante para entregar o que já estava no pipe.
pub(crate) fn follow(
    app: &AppHandle,
    tx: u32,
    command: Command,
    source: &str,
    failed: String,
) -> bool {
    follow_then(app, tx, command, source, failed, |_| {})
}

/// `follow` com um passo final que roda antes do evento `Finished`: o que
/// muda o estado mostrado (registro de instalados, atalhos) precisa estar
/// pronto quando a interface recarrega ao ver o fim da tarefa.
pub(crate) fn follow_then(
    app: &AppHandle,
    tx: u32,
    mut command: Command,
    source: &str,
    failed: String,
    before_finish: impl FnOnce(bool),
) -> bool {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            before_finish(false);
            emit(TransactionEvent::Finished {
                id: tx,
                success: false,
                message: error.to_string(),
            });
            return false;
        }
    };
    let last_error = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    if let Some(stderr) = child.stderr.take() {
        let err_app = app.clone();
        let last_error = last_error.clone();
        let done = done_tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(mut last) = last_error.lock() {
                    last.clone_from(&line);
                }
                let _ = err_app.emit(
                    "software-transaction",
                    TransactionEvent::Console {
                        id: tx,
                        source: "stderr".into(),
                        line,
                    },
                );
            }
            let _ = done.send(());
        });
    }
    if let Some(stdout) = child.stdout.take() {
        let out_app = app.clone();
        let source = source.to_owned();
        let done = done_tx.clone();
        std::thread::spawn(move || {
            // O --noninteractive não informa porcentagem: cada etapa (runtime,
            // extensão, app) avança a barra, sem chegar a 100 antes do fim.
            let mut steps = 0u32;
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let line = line.trim().to_owned();
                if line.is_empty() {
                    continue;
                }
                // "flatpak", não "stdout": a interface ignora o stdout do
                // vegad, que repete os eventos de pacote.
                let _ = out_app.emit(
                    "software-transaction",
                    TransactionEvent::Console {
                        id: tx,
                        source: source.clone(),
                        line: line.clone(),
                    },
                );
                steps += 1;
                let _ = out_app.emit(
                    "software-transaction",
                    TransactionEvent::Progress {
                        id: tx,
                        percent: step_percent(steps),
                        message: line,
                    },
                );
            }
            let _ = done.send(());
        });
    }
    drop(done_tx);
    let success = child.wait().map(|status| status.success()).unwrap_or(false);
    // Até duas leituras terminando, por no máximo 2 s no total.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    for _ in 0..2 {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if done_rx.recv_timeout(left).is_err() {
            break;
        }
    }
    let last_error = last_error
        .lock()
        .map(|last| last.clone())
        .unwrap_or_default();
    before_finish(success);
    emit(TransactionEvent::Finished {
        id: tx,
        success,
        message: if success {
            gettext("Concluído")
        } else if last_error.is_empty() {
            failed
        } else {
            last_error
        },
    });
    success
}

/// Progresso por etapa: sobe rápido no começo e para em 95%.
pub(crate) fn step_percent(steps: u32) -> u32 {
    95 - 90 * 4 / (4 + steps)
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
    fn arguments_keep_the_id_after_the_option_terminator() {
        assert_eq!(
            arguments(Operation::Install, "us.zoom.Zoom", "user"),
            [
                "install",
                "-y",
                "--noninteractive",
                "--app",
                "--system",
                "--",
                "flathub",
                "us.zoom.Zoom"
            ]
        );
        assert_eq!(
            arguments(Operation::Update, "us.zoom.Zoom", "user"),
            [
                "update",
                "-y",
                "--noninteractive",
                "--app",
                "--user",
                "--",
                "us.zoom.Zoom"
            ]
        );
        assert_eq!(
            arguments(Operation::Remove, "us.zoom.Zoom", "system"),
            [
                "uninstall",
                "-y",
                "--noninteractive",
                "--app",
                "--system",
                "--",
                "us.zoom.Zoom"
            ]
        );
    }

    #[test]
    fn step_progress_grows_and_stays_below_done() {
        let values: Vec<u32> = (0..40).map(step_percent).collect();
        assert_eq!(values[0], 5);
        assert!(values.windows(2).all(|w| w[0] <= w[1]));
        assert!(values.iter().all(|&v| v < 100));
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
