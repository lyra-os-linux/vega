//! Máquinas virtuais via libvirt (crate vega-virtualization), sem vegad.
//!
//! O libvirt faz a autorização; cada operação abre a própria conexão numa
//! thread de bloqueio, como no vega-gtk. A conexão vem em cada pedido: a
//! interface só aplica o resultado se o usuário ainda estiver na mesma.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use vega_virtualization::{
    Action, Backend, Connection, CreateRequest, Firmware, Machine, MediaKind, RemovalPlan, State,
};

use crate::virtualization_messages::translate;

/// Criação em andamento: pedido de cancelamento e trava de uma por vez.
#[derive(Default)]
pub struct Creation {
    pub cancel: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}

/// Planos de remoção mostrados ao usuário, por conexão + uuid. A remoção com
/// arquivos só usa exatamente o plano revisado (o crate confere de novo).
static PLANS: Mutex<Option<HashMap<String, RemovalPlan>>> = Mutex::new(None);

#[derive(Deserialize)]
struct Target {
    connection: String,
    uuid: String,
}

#[derive(Deserialize)]
struct CreateArgs {
    name: String,
    source: String,
    kind: String,
    firmware: String,
    cpus: u32,
    memory_mib: u64,
    disk_gib: u64,
}

fn connection(id: &str) -> Result<Connection, String> {
    match id {
        "personal" => Ok(Connection::Personal),
        "system" => Ok(Connection::System),
        other => Err(format!("conexão desconhecida: {other}")),
    }
}

fn action(id: &str) -> Result<Action, String> {
    Ok(match id {
        "start" => Action::Start,
        "shutdown" => Action::Shutdown,
        "pause" => Action::Pause,
        "resume" => Action::Resume,
        "force-stop" => Action::ForceStop,
        "remove" => Action::Remove,
        other => return Err(format!("ação desconhecida: {other}")),
    })
}

const ACTIONS: [(&str, Action); 6] = [
    ("start", Action::Start),
    ("shutdown", Action::Shutdown),
    ("pause", Action::Pause),
    ("resume", Action::Resume),
    ("force-stop", Action::ForceStop),
    ("remove", Action::Remove),
];

fn machine(m: Machine) -> Value {
    let state = match m.state {
        State::Running => "running",
        State::Paused => "paused",
        State::Off => "off",
        State::Other => "other",
    };
    let actions: Vec<&str> = ACTIONS
        .iter()
        .filter(|(_, a)| a.allowed(m.state, m.persistent))
        .map(|(id, _)| *id)
        .collect();
    json!({
        "uuid": m.uuid,
        "name": m.name,
        "state": state,
        "cpus": m.cpus,
        "memory_mib": m.memory_kib / 1024,
        "persistent": m.persistent,
        "actions": actions,
        "editable": m.state == State::Off && m.persistent,
    })
}

fn args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// Roda uma operação do libvirt fora da thread do runtime e traduz os erros.
async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|_| crate::i18n::gettext("A operação foi interrompida."))?
        .map_err(|error| translate(&error))
}

fn plan_key(connection: Connection, uuid: &str) -> String {
    format!("{}#{uuid}", connection.uri())
}

#[tauri::command]
pub async fn vms_call(
    app: AppHandle,
    creation: tauri::State<'_, Creation>,
    op: String,
    args: Value,
) -> Result<Value, String> {
    match op.as_str() {
        "list" => {
            #[derive(Deserialize)]
            struct List {
                connection: String,
            }
            let conn = connection(&self::args::<List>(args)?.connection)?;
            let machines = blocking(move || Backend::open(conn, true)?.list()).await?;
            Ok(Value::Array(machines.into_iter().map(machine).collect()))
        }
        "act" => {
            #[derive(Deserialize)]
            struct Act {
                connection: String,
                uuid: String,
                action: String,
            }
            let request: Act = self::args(args)?;
            let conn = connection(&request.connection)?;
            let act = action(&request.action)?;
            blocking(move || Backend::open(conn, false)?.act(&request.uuid, act)).await?;
            Ok(Value::Null)
        }
        "open-console" => {
            let target: Target = self::args(args)?;
            let conn = connection(&target.connection)?;
            uuid_check(&target.uuid)?;
            let mut child = std::process::Command::new("lyra-vms")
                .args(["--connect", conn.uri(), "--uuid", &target.uuid])
                .spawn()
                .map_err(|error| {
                    format!(
                        "{}\n{error}",
                        crate::i18n::gettext(
                            "Não foi possível abrir o Lyra VMs. Confira se o pacote está instalado."
                        )
                    )
                })?;
            // Recolhe o processo quando o console fechar.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            Ok(Value::Null)
        }
        "shortcut" => {
            #[derive(Deserialize)]
            struct Shortcut {
                connection: String,
                uuid: String,
                name: String,
            }
            let request: Shortcut = self::args(args)?;
            let conn = connection(&request.connection)?;
            blocking(move || {
                let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
                let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
                vega_virtualization::create_shortcut(
                    conn,
                    &request.uuid,
                    &request.name,
                    xdg.as_deref(),
                    &PathBuf::from(home),
                )
            })
            .await?;
            Ok(Value::Null)
        }
        "removal-plan" => {
            let target: Target = self::args(args)?;
            let conn = connection(&target.connection)?;
            let uuid = target.uuid.clone();
            let plan = blocking(move || Backend::open(conn, true)?.removal_plan(&uuid)).await?;
            let files = plan.files.clone();
            if let Ok(mut plans) = PLANS.lock() {
                plans
                    .get_or_insert_with(HashMap::new)
                    .insert(plan_key(conn, &target.uuid), plan);
            }
            Ok(json!({ "files": files }))
        }
        "remove-with-files" => {
            let target: Target = self::args(args)?;
            let conn = connection(&target.connection)?;
            let plan = PLANS
                .lock()
                .ok()
                .and_then(|mut plans| {
                    plans
                        .as_mut()
                        .and_then(|p| p.remove(&plan_key(conn, &target.uuid)))
                })
                .ok_or_else(|| {
                    translate("The local file list changed. Review the removal again.")
                })?;
            blocking(move || Backend::open(conn, false)?.remove_with_files(&target.uuid, &plan))
                .await?;
            Ok(Value::Null)
        }
        "edit-details" => {
            let target: Target = self::args(args)?;
            let conn = connection(&target.connection)?;
            let details =
                blocking(move || Backend::open(conn, false)?.edit_details(&target.uuid)).await?;
            let disks: Vec<Value> = details
                .disks
                .into_iter()
                .map(|disk| {
                    json!({
                        "target": disk.target,
                        "path": disk.path,
                        "capacity": disk.capacity,
                        "unavailable": disk.unavailable.map(|reason| translate(&reason)),
                    })
                })
                .collect();
            Ok(json!({ "disks": disks, "media": details.media }))
        }
        "rename" => {
            #[derive(Deserialize)]
            struct Rename {
                connection: String,
                uuid: String,
                name: String,
            }
            let request: Rename = self::args(args)?;
            let conn = connection(&request.connection)?;
            blocking(move || Backend::open(conn, false)?.rename(&request.uuid, &request.name))
                .await?;
            Ok(Value::Null)
        }
        "configure" => {
            #[derive(Deserialize)]
            struct Configure {
                connection: String,
                uuid: String,
                cpus: u32,
                memory_mib: u64,
            }
            let request: Configure = self::args(args)?;
            let conn = connection(&request.connection)?;
            blocking(move || {
                Backend::open(conn, false)?.configure(
                    &request.uuid,
                    request.cpus,
                    request.memory_mib,
                )
            })
            .await?;
            Ok(Value::Null)
        }
        "grow-disk" => {
            #[derive(Deserialize)]
            struct Grow {
                connection: String,
                uuid: String,
                target: String,
                gib: u64,
            }
            let request: Grow = self::args(args)?;
            let conn = connection(&request.connection)?;
            blocking(move || {
                Backend::open(conn, false)?.grow_disk(
                    &request.uuid,
                    &request.target,
                    request.gib << 30,
                )
            })
            .await?;
            Ok(Value::Null)
        }
        "eject-iso" => {
            #[derive(Deserialize)]
            struct Eject {
                connection: String,
                uuid: String,
                target: String,
            }
            let request: Eject = self::args(args)?;
            let conn = connection(&request.connection)?;
            blocking(move || Backend::open(conn, false)?.eject_iso(&request.uuid, &request.target))
                .await?;
            Ok(Value::Null)
        }
        "create" => create(app, &creation, self::args(args)?).await,
        "cancel-create" => {
            creation.cancel.store(true, Ordering::Relaxed);
            Ok(Value::Null)
        }
        _ => Err(format!("operação de máquinas virtuais desconhecida: {op}")),
    }
}

fn uuid_check(uuid: &str) -> Result<(), String> {
    // A uuid vai como argumento de outro programa: aceite só o formato canônico.
    let ok = uuid.len() == 36
        && uuid.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        });
    ok.then_some(()).ok_or_else(|| "uuid inválida".to_owned())
}

/// Cria uma máquina pessoal, emitindo `vms-create-progress` ({ done, total })
/// no máximo a cada 100 ms. A máquina termina desligada.
async fn create(app: AppHandle, creation: &Creation, request: CreateArgs) -> Result<Value, String> {
    let request = CreateRequest {
        name: request.name,
        source: PathBuf::from(request.source),
        kind: match request.kind.as_str() {
            "qcow2" => MediaKind::Qcow2,
            "raw" => MediaKind::Raw,
            _ => MediaKind::Iso,
        },
        firmware: if request.firmware == "uefi" {
            Firmware::Uefi
        } else {
            Firmware::Bios
        },
        cpus: request.cpus,
        memory_mib: request.memory_mib,
        disk_gib: request.disk_gib,
    };
    request.validate().map_err(|error| translate(&error))?;
    if creation.running.swap(true, Ordering::AcqRel) {
        return Err(crate::i18n::gettext("Executando operação…"));
    }
    creation.cancel.store(false, Ordering::Relaxed);
    let cancel = creation.cancel.clone();
    let running = creation.running.clone();
    let result = blocking(move || {
        let mut last = Instant::now() - Duration::from_secs(1);
        Backend::open(Connection::Personal, false)?.create_with_progress(
            &request,
            &cancel,
            |done, total| {
                if last.elapsed() >= Duration::from_millis(100) || done >= total {
                    last = Instant::now();
                    let _ = app.emit(
                        "vms-create-progress",
                        json!({ "done": done, "total": total }),
                    );
                }
            },
        )
    })
    .await;
    running.store(false, Ordering::Release);
    result.map(Value::String)
}
