//! Teste de GPU do diagnóstico (vega#165, teste 7). Roda aqui, na sessão do
//! usuário, porque o vegad (root, sem sessão gráfica) não alcança o EGL e o
//! Wayland do desktop. O resultado tem a mesma forma de uma etapa do
//! relatório do vegad (`id`, `verdict`, `summary`, `findings`, `metrics`),
//! para a página juntar tudo num relatório só.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gettextrs::gettext;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use super::CmdResult;

/// Uma GPU como o `vulkaninfo --summary` a descreve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpu {
    pub name: String,
    pub driver: String,
    pub kind: GpuKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuKind {
    Integrated,
    Discrete,
    Software,
}

/// Lê os blocos `GPUn:` do `vulkaninfo --summary`.
pub fn parse_vulkan_summary(text: &str) -> Vec<Gpu> {
    let mut gpus = Vec::new();
    let mut current: Option<Gpu> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GPU") && trimmed.ends_with(':') {
            gpus.extend(current.take());
            current = Some(Gpu {
                name: String::new(),
                driver: String::new(),
                kind: GpuKind::Software,
            });
            continue;
        }
        let (Some(gpu), Some((key, value))) = (current.as_mut(), trimmed.split_once('=')) else {
            continue;
        };
        let value = value.trim().to_owned();
        match key.trim() {
            "deviceName" => gpu.name = value,
            "driverName" => gpu.driver = value,
            "deviceType" => {
                gpu.kind = match value.as_str() {
                    "PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU" => GpuKind::Integrated,
                    "PHYSICAL_DEVICE_TYPE_DISCRETE_GPU" => GpuKind::Discrete,
                    _ => GpuKind::Software,
                }
            }
            _ => {}
        }
    }
    gpus.extend(current);
    gpus
}

/// GPUs a medir: as de hardware; o llvmpipe só quando não há nenhuma (VM).
pub fn bench_targets(gpus: &[Gpu]) -> Vec<Gpu> {
    let hardware: Vec<Gpu> = gpus
        .iter()
        .filter(|gpu| gpu.kind != GpuKind::Software)
        .cloned()
        .collect();
    if hardware.is_empty() {
        gpus.iter().take(1).cloned().collect()
    } else {
        hardware
    }
}

/// O renderer EGL de cada plataforma do `eglinfo -B`, como "Wayland: Mesa …".
pub fn parse_egl_renderers(text: &str) -> Vec<String> {
    let mut renderers = Vec::new();
    let mut platform = String::new();
    for line in text.lines() {
        if let Some(name) = line.strip_suffix(" platform:") {
            platform = name.trim_start_matches("Platform ").to_owned();
        } else if let Some(renderer) = line.strip_prefix("OpenGL ES profile renderer:") {
            let entry = format!("{platform}: {}", renderer.trim());
            if !renderers.contains(&entry) {
                renderers.push(entry);
            }
        }
    }
    renderers
}

/// Pontuação final do glmark2 e cenas descartadas pelo erro conhecido da
/// NVIDIA (C7101, macro redefinida no compilador de shaders).
pub fn parse_glmark2(text: &str) -> (Option<u32>, usize) {
    let score = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("glmark2 Score:")
            .and_then(|value| value.trim().parse().ok())
    });
    let known_failures = text.matches("error C7101").count();
    (score, known_failures)
}

/// Variáveis que mandam o glmark2 para a GPU certa num notebook híbrido.
pub fn offload_env(gpu: &Gpu) -> Vec<(&'static str, &'static str)> {
    if gpu.kind != GpuKind::Discrete {
        return Vec::new();
    }
    if gpu.driver == "NVIDIA" {
        vec![
            ("__NV_PRIME_RENDER_OFFLOAD", "1"),
            (
                "__EGL_VENDOR_LIBRARY_FILENAMES",
                "/usr/share/glvnd/egl_vendor.d/10_nvidia.json",
            ),
        ]
    } else {
        vec![("DRI_PRIME", "1")]
    }
}

fn on_line_power() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return true;
    };
    let mut saw_mains = false;
    for entry in entries.flatten() {
        let read = |name: &str| {
            std::fs::read_to_string(entry.path().join(name))
                .map(|value| value.trim().to_owned())
                .unwrap_or_default()
        };
        if read("type") == "Mains" {
            saw_mains = true;
            if read("online") == "1" {
                return true;
            }
        }
    }
    !saw_mains
}

/// As placas NVIDIA com gerenciamento de energia em tempo de execução ligado
/// no kernel (`power/control` = `auto`), em ordem de endereço PCI.
fn nvidia_runtime_pm(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join("sys/bus/pci/devices")) else {
        return Vec::new();
    };
    let mut devices: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let read = |name: &str| std::fs::read_to_string(path.join(name)).unwrap_or_default();
            read("vendor").trim() == "0x10de"
                && read("class").starts_with("0x03")
                && read("power/control").trim() == "auto"
        })
        .collect();
    devices.sort();
    devices
}

/// O módulo de kernel ligado à placa (`nvidia`, `nouveau`...), se houver.
fn pci_driver(device: &Path) -> Option<String> {
    let target = std::fs::read_link(device.join("driver")).ok()?;
    Some(target.file_name()?.to_string_lossy().into_owned())
}

/// Estado do desligamento automático (RTD3, Runtime D3) que o driver
/// `nvidia` informa em `/proc/driver/nvidia/gpus/<endereço>/power`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rtd3 {
    Fine,
    Coarse,
    Unsupported,
    Disabled,
    Unknown,
}

impl Rtd3 {
    pub fn as_str(self) -> &'static str {
        match self {
            Rtd3::Fine => "fine",
            Rtd3::Coarse => "coarse",
            Rtd3::Unsupported => "unsupported",
            Rtd3::Disabled => "disabled",
            Rtd3::Unknown => "unknown",
        }
    }
}

/// Classifica o conteúdo do arquivo `power` do driver. Só vale a linha cuja
/// chave é exatamente `Runtime D3 status`; o `Status:` da seção S0ix não conta.
pub fn classify_rtd3(power: &str) -> Rtd3 {
    let Some(value) = power.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "Runtime D3 status").then(|| value.trim())
    }) else {
        return Rtd3::Unknown;
    };
    if value.starts_with("Enabled (fine-grained)") {
        Rtd3::Fine
    } else if value.starts_with("Enabled (coarse-grained)") {
        Rtd3::Coarse
    } else if value.starts_with("Not supported") {
        Rtd3::Unsupported
    } else if value.starts_with("Disabled") {
        Rtd3::Disabled
    } else {
        Rtd3::Unknown
    }
}

/// Lê o estado RTD3 de uma placa. Ler o `/proc` do driver não acorda a GPU.
pub fn read_rtd3(root: &Path, address: &str) -> Rtd3 {
    std::fs::read_to_string(
        root.join("proc/driver/nvidia/gpus")
            .join(address)
            .join("power"),
    )
    .map_or(Rtd3::Unknown, |power| classify_rtd3(&power))
}

/// Processos que ficam com a placa aberta de propósito e não indicam defeito.
const EXPECTED_HOLDERS: [&str; 6] = [
    "gnome-shell",
    "kwin_wayland",
    "kwin_x11",
    "Xorg",
    "Xwayland",
    "nvidia-persistenced",
];

/// Até quantos processos o `raw` lista.
const MAX_HOLDERS: usize = 20;

/// Nó de dispositivo da NVIDIA ou nó DRM (`card*`, `renderD*`) da placa?
fn is_gpu_node(target: &Path, drm_nodes: &[PathBuf]) -> bool {
    if drm_nodes.iter().any(|node| node == target) {
        return true;
    }
    let Some(name) = target
        .strip_prefix("/dev")
        .ok()
        .and_then(|rest| rest.to_str())
    else {
        return false;
    };
    matches!(
        name,
        "nvidiactl" | "nvidia-modeset" | "nvidia-uvm" | "nvidia-uvm-tools"
    ) || name
        .strip_prefix("nvidia")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Nome do processo: o executável, porque o `comm` corta em 15 caracteres.
fn process_name(proc_dir: &Path) -> Option<String> {
    if let Ok(exe) = std::fs::read_link(proc_dir.join("exe"))
        && let Some(name) = exe.file_name()
    {
        let name = name.to_string_lossy();
        return Some(name.strip_suffix(" (deleted)").unwrap_or(&name).to_owned());
    }
    std::fs::read_to_string(proc_dir.join("comm"))
        .ok()
        .map(|comm| comm.trim().to_owned())
}

/// Processos com a placa aberta, por `/proc/<pid>/fd`, sem tocar a GPU.
/// Ignora o compositor, o persistenced e o próprio Vega (`self_pid`). Como o
/// Vega roda na sessão do usuário, os processos de outros usuários não podem
/// ser lidos e ficam de fora.
pub fn gpu_holders(root: &Path, address: &str, self_pid: u32) -> Vec<(u32, String)> {
    let drm_nodes: Vec<PathBuf> =
        std::fs::read_dir(root.join("sys/bus/pci/devices").join(address).join("drm"))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.starts_with("card") || name.starts_with("renderD"))
                    .map(|name| Path::new("/dev/dri").join(name))
                    .collect()
            })
            .unwrap_or_default();
    let Ok(procs) = std::fs::read_dir(root.join("proc")) else {
        return Vec::new();
    };
    let mut holders: Vec<(u32, String)> = procs
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            if pid == self_pid {
                return None;
            }
            // Processos que terminaram ou de outro usuário dão erro: ignorar.
            let opens_gpu = std::fs::read_dir(entry.path().join("fd"))
                .ok()?
                .flatten()
                .filter_map(|fd| std::fs::read_link(fd.path()).ok())
                .any(|target| is_gpu_node(&target, &drm_nodes));
            if !opens_gpu {
                return None;
            }
            let name = process_name(&entry.path())?;
            (!EXPECTED_HOLDERS.contains(&name.as_str())).then_some((pid, name))
        })
        .collect();
    holders.sort();
    holders.truncate(MAX_HOLDERS);
    holders
}

/// Achados de RTD3 de uma placa com o driver `nvidia`. `suspended` é o
/// `runtime_status` depois da espera, que só é feita no modo `fine`.
pub fn rtd3_findings(
    state: Rtd3,
    suspended: bool,
    holders: &[(u32, String)],
    sysfs: &str,
) -> Vec<Value> {
    let raw = || {
        if holders.is_empty() {
            sysfs.to_owned()
        } else {
            holders
                .iter()
                .map(|(pid, name)| format!("{pid} {name}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    let found = match state {
        Rtd3::Fine if suspended => return Vec::new(),
        Rtd3::Fine => finding(
            "attention",
            "gpu.rtd3-awake",
            if holders.is_empty() {
                gettext(
                    "A NVIDIA não voltou a economizar energia 30 s depois do teste, sem nenhum programa identificado usando a placa",
                )
            } else {
                gettext(
                    "A NVIDIA não voltou a economizar energia 30 s depois do teste; os programas listados estão com a placa aberta",
                )
            },
            raw(),
        ),
        Rtd3::Coarse => finding(
            "info",
            "gpu.rtd3-coarse",
            gettext(
                "A NVIDIA só desliga quando nenhum programa a mantém aberta, e o ambiente gráfico a mantém; ela fica em modo de economia. Não é defeito",
            ),
            raw(),
        ),
        Rtd3::Unsupported => finding(
            "info",
            "gpu.rtd3-unsupported",
            gettext(
                "O driver instalado não desliga esta placa NVIDIA quando ociosa; ela fica em modo de economia, o que reduz um pouco a duração da bateria. Não é defeito",
            ),
            sysfs.to_owned(),
        ),
        Rtd3::Disabled => finding(
            "info",
            "gpu.rtd3-disabled",
            gettext(
                "O desligamento automático da NVIDIA está desativado no driver; a placa fica em modo de economia quando ociosa. Não é defeito",
            ),
            sysfs.to_owned(),
        ),
        Rtd3::Unknown => finding(
            "info",
            "gpu.rtd3-unknown",
            gettext(
                "Não foi possível ler o estado de economia de energia da NVIDIA informado pelo driver",
            ),
            sysfs.to_owned(),
        ),
    };
    vec![found]
}

fn runtime_suspended(device: &Path) -> bool {
    std::fs::read_to_string(device.join("power/runtime_status"))
        .is_ok_and(|status| status.trim() == "suspended")
}

/// Espera a placa suspender até o prazo, comum a todas as placas.
fn wait_suspended(device: &Path, deadline: Instant) -> bool {
    loop {
        if runtime_suspended(device) {
            return true;
        }
        if Instant::now() >= deadline || CANCELLED.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Uma métrica por placa: o valor direto com uma placa, objeto indexado pelo
/// endereço PCI com mais de uma (como `nvme0n1` em `disk-health`).
fn per_device(values: serde_json::Map<String, Value>) -> Value {
    if values.len() == 1 {
        values
            .into_iter()
            .next()
            .map(|(_, value)| value)
            .unwrap_or(Value::Null)
    } else {
        Value::Object(values)
    }
}

/// Verificação de energia das NVIDIA depois do teste. Não pode abrir
/// `/dev/nvidia*` nem chamar ferramentas da NVIDIA: isso acorda a placa e
/// invalida a medição. Só lê sysfs e `/proc`.
fn nvidia_power_check(
    root: &Path,
    progress: &dyn Fn(u32, String),
    findings: &mut Vec<Value>,
    metrics: &mut Value,
) {
    let devices = nvidia_runtime_pm(root);
    if devices.is_empty() {
        return;
    }
    let states: Vec<(PathBuf, String, Option<Rtd3>)> = devices
        .into_iter()
        .map(|device| {
            let address = device
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            // Outros drivers (nouveau) seguem a regra anterior: esperar e
            // conferir o runtime_status.
            let state = (pci_driver(&device).as_deref() == Some("nvidia"))
                .then(|| read_rtd3(root, &address));
            (device, address, state)
        })
        .collect();
    let needs_wait = states
        .iter()
        .any(|(_, _, state)| matches!(state, None | Some(Rtd3::Fine)));
    if needs_wait {
        progress(92, gettext("Conferindo se a NVIDIA volta a dormir"));
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut suspended_by_device = serde_json::Map::new();
    let mut rtd3_by_device = serde_json::Map::new();
    for (device, address, state) in states {
        let sysfs = device.display().to_string();
        let suspended = match state {
            None | Some(Rtd3::Fine) => wait_suspended(&device, deadline),
            Some(_) => runtime_suspended(&device),
        };
        if CANCELLED.load(Ordering::Relaxed) {
            return;
        }
        suspended_by_device.insert(address.clone(), json!(suspended));
        match state {
            None => {
                if !suspended {
                    findings.push(finding(
                        "attention",
                        "gpu.rtd3-awake",
                        gettext("A NVIDIA não voltou a economizar energia 30 s depois do teste; algum programa pode estar usando a GPU"),
                        sysfs,
                    ));
                }
            }
            Some(state) => {
                rtd3_by_device.insert(address.clone(), json!(state.as_str()));
                let lists_holders = state == Rtd3::Coarse || (state == Rtd3::Fine && !suspended);
                let holders = if lists_holders {
                    gpu_holders(root, &address, std::process::id())
                } else {
                    Vec::new()
                };
                findings.extend(rtd3_findings(state, suspended, &holders, &sysfs));
            }
        }
    }
    metrics["nvidiaRuntimeSuspended"] = per_device(suspended_by_device);
    if !rtd3_by_device.is_empty() {
        metrics["nvidiaRTD3"] = per_device(rtd3_by_device);
    }
}

/// Duração de cada cena do glmark2, aplicada a toda a lista padrão.
const GLMARK2_SCENE_DURATION: &str = ":duration=3";

static CANCELLED: AtomicBool = AtomicBool::new(false);
static RUNNING: Mutex<Option<Child>> = Mutex::new(None);

fn tool(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

/// Roda uma ferramenta gráfica sem herdar o contorno do WebKit (vega#166)
/// e com prazo; o cancelamento da página mata o processo.
fn run(program: &str, args: &[&str], envs: &[(&str, &str)], timeout: Duration) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(envs.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::webkit_env::clean_command(&mut command);
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        let mut err = String::new();
        let _ = stderr.read_to_string(&mut err);
        out + &err
    });
    *RUNNING.lock().unwrap_or_else(|p| p.into_inner()) = Some(child);
    let start = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let mut slot = RUNNING.lock().unwrap_or_else(|p| p.into_inner());
        let Some(child) = slot.as_mut() else { break };
        if CANCELLED.load(Ordering::Relaxed) || start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            *slot = None;
            break;
        }
        if child.try_wait().ok().flatten().is_some() {
            *slot = None;
            break;
        }
    }
    reader.join().ok()
}

#[derive(Debug, Clone, Serialize)]
struct GpuProgress {
    percent: u32,
    message: String,
}

fn finding(severity: &str, code: &str, text: String, raw: String) -> Value {
    json!({ "severity": severity, "code": code, "text": text, "raw": raw })
}

/// Executa o teste e devolve a etapa `gpu` do relatório.
#[tauri::command]
pub async fn gpu_bench(app: AppHandle) -> CmdResult<Value> {
    if !on_line_power() {
        return Err(gettext("Conecte o computador à tomada para testar a GPU."));
    }
    for (program, package) in [
        ("vulkaninfo", "vulkan-tools"),
        ("eglinfo", "Mesa-demo-egl"),
        ("glmark2-es2-wayland", "glmark2"),
    ] {
        if !tool(program) {
            return Ok(json!({
                "id": "gpu", "verdict": "unavailable",
                "summary": gettext("Indisponível neste sistema"),
                "durationSeconds": 0, "findings": [],
                "metrics": { "reason": format!("missing-tool:{package}") },
            }));
        }
    }
    CANCELLED.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || bench(&app))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn gpu_bench_cancel() {
    CANCELLED.store(true, Ordering::Relaxed);
}

fn bench(app: &AppHandle) -> Value {
    let start = Instant::now();
    let progress = |percent: u32, message: String| {
        let _ = app.emit("diagnostics-gpu", GpuProgress { percent, message });
    };
    progress(2, gettext("Detectando as GPUs"));
    let vulkan =
        run("vulkaninfo", &["--summary"], &[], Duration::from_secs(30)).unwrap_or_default();
    let gpus = parse_vulkan_summary(&vulkan);
    let egl = run("eglinfo", &["-B"], &[], Duration::from_secs(30)).unwrap_or_default();
    let renderers = parse_egl_renderers(&egl);
    let targets = bench_targets(&gpus);

    let mut findings = Vec::new();
    let mut scores = serde_json::Map::new();
    let mut cancelled = false;
    for (index, gpu) in targets.iter().enumerate() {
        let base = 5 + 85 * index as u32 / targets.len().max(1) as u32;
        progress(base, gettext("Medindo {gpu}").replace("{gpu}", &gpu.name));
        let env = offload_env(gpu);
        // 3 s por cena (o padrão é 10 s): as 33 cenas levam ~100 s por GPU
        // em vez de ~6 min; a pontuação continua comparável entre execuções.
        let out = run(
            "glmark2-es2-wayland",
            &["--off-screen", "-b", GLMARK2_SCENE_DURATION],
            &env,
            Duration::from_secs(240),
        )
        .unwrap_or_default();
        if CANCELLED.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let (score, known) = parse_glmark2(&out);
        match score {
            Some(score) => {
                scores.insert(gpu.name.clone(), json!(score));
            }
            None => findings.push(finding(
                "attention",
                "gpu.bench-failed",
                gettext("Não foi possível medir {gpu}").replace("{gpu}", &gpu.name),
                out.lines()
                    .rev()
                    .take(12)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
        }
        if known > 0 {
            findings.push(finding(
                "info",
                "gpu.glmark2-nvidia-shader",
                gettext("{count} cenas do glmark2 não rodam no compilador da NVIDIA (incompatibilidade conhecida, não é defeito)")
                    .replace("{count}", &known.to_string()),
                String::new(),
            ));
        }
    }

    let mut metrics = json!({
        "gpus": gpus.iter().map(|gpu| json!({"name": gpu.name, "driver": gpu.driver, "type": format!("{:?}", gpu.kind).to_lowercase()})).collect::<Vec<_>>(),
        "eglRenderers": renderers,
        "offscreenScores": scores,
    });
    if !cancelled {
        nvidia_power_check(Path::new("/"), &progress, &mut findings, &mut metrics);
    }
    let worst = if findings.iter().any(|f| f["severity"] == "fail") {
        "fail"
    } else if findings.iter().any(|f| f["severity"] == "attention") {
        "attention"
    } else {
        "ok"
    };
    let verdict = if cancelled { "cancelled" } else { worst };
    let summary = if cancelled {
        gettext("Cancelado")
    } else {
        scores
            .iter()
            .map(|(name, score)| format!("{name}: {score}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    progress(100, summary.clone());
    json!({
        "id": "gpu",
        "verdict": verdict,
        "summary": summary,
        "durationSeconds": start.elapsed().as_secs(),
        "findings": findings,
        "metrics": metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VULKAN: &str = include_str!("fixtures/vulkaninfo-summary.txt");
    const EGL: &str = include_str!("fixtures/eglinfo-B.txt");

    #[test]
    fn hybrid_laptop_benches_both_gpus_but_not_llvmpipe() {
        let gpus = parse_vulkan_summary(VULKAN);
        assert_eq!(gpus.len(), 3);
        assert_eq!(gpus[0].name, "Intel(R) UHD Graphics (TGL GT1)");
        assert_eq!(gpus[0].kind, GpuKind::Integrated);
        assert_eq!(gpus[1].driver, "NVIDIA");
        assert_eq!(gpus[1].kind, GpuKind::Discrete);
        assert_eq!(gpus[2].kind, GpuKind::Software);
        let targets = bench_targets(&gpus);
        assert_eq!(targets.len(), 2);
        assert!(offload_env(&targets[0]).is_empty());
        assert_eq!(
            offload_env(&targets[1])[0],
            ("__NV_PRIME_RENDER_OFFLOAD", "1")
        );
    }

    #[test]
    fn a_vm_without_gpu_falls_back_to_llvmpipe() {
        let gpus = vec![Gpu {
            name: "llvmpipe".into(),
            driver: "llvmpipe".into(),
            kind: GpuKind::Software,
        }];
        assert_eq!(bench_targets(&gpus).len(), 1);
        let amd = Gpu {
            name: "AMD".into(),
            driver: "radv".into(),
            kind: GpuKind::Discrete,
        };
        assert_eq!(offload_env(&amd), vec![("DRI_PRIME", "1")]);
    }

    #[test]
    fn egl_renderers_per_platform() {
        let renderers = parse_egl_renderers(EGL);
        assert!(renderers.contains(&"Wayland: Mesa Intel(R) UHD Graphics (TGL GT1)".to_owned()));
        assert!(renderers.contains(&"Surfaceless: NVIDIA GeForce GTX 1650/PCIe/SSE2".to_owned()));
        assert!(renderers.contains(&"Device: NVIDIA GeForce GTX 1650/PCIe/SSE2".to_owned()));
    }

    #[test]
    fn glmark2_score_and_known_nvidia_noise() {
        assert_eq!(
            parse_glmark2(include_str!("fixtures/glmark2-intel.txt")),
            (Some(4394), 0)
        );
        assert_eq!(
            parse_glmark2(include_str!("fixtures/glmark2-nvidia.txt")),
            (Some(2648), 3)
        );
        assert_eq!(parse_glmark2("Error: no EGL"), (None, 0));
    }

    /// Saída real do Nitro AN515-57 (GTX 1650, G07 595.99.02, módulo aberto)
    /// com `NVreg_DynamicPowerManagement=0x02`.
    const NITRO_POWER: &str = include_str!("fixtures/nvidia-power-nitro-unsupported.txt");

    fn with_first_line(value: &str) -> String {
        let rest = NITRO_POWER.split_once('\n').map_or("", |(_, rest)| rest);
        format!("Runtime D3 status:          {value}\n{rest}")
    }

    #[test]
    fn classifies_the_rtd3_state_reported_by_the_driver() {
        assert_eq!(classify_rtd3(NITRO_POWER), Rtd3::Unsupported);
        let cases = [
            ("Disabled by default", Rtd3::Disabled),
            ("Disabled", Rtd3::Disabled),
            ("Enabled (fine-grained)", Rtd3::Fine),
            ("Enabled (coarse-grained)", Rtd3::Coarse),
            ("Not supported", Rtd3::Unsupported),
            ("Something new", Rtd3::Unknown),
        ];
        for (value, expected) in cases {
            assert_eq!(classify_rtd3(&with_first_line(value)), expected, "{value}");
        }
        assert_eq!(classify_rtd3(""), Rtd3::Unknown);
    }

    #[test]
    fn the_s0ix_status_line_is_not_the_rtd3_state() {
        // Sem a linha do RTD3 sobram "Status: Disabled" (S0ix) e outros
        // "Disabled", que não podem virar `disabled`.
        let without: String = NITRO_POWER
            .lines()
            .filter(|line| !line.starts_with("Runtime D3 status"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert!(without.contains(" Status:                    Disabled"));
        assert_eq!(classify_rtd3(&without), Rtd3::Unknown);
        // Com fine-grained na primeira linha, o Disabled do S0ix não vence.
        assert_eq!(
            classify_rtd3(&with_first_line("Enabled (fine-grained)")),
            Rtd3::Fine
        );
    }

    /// Diretório temporário com uma árvore `/proc` e `/sys` falsa.
    struct FakeRoot(PathBuf);

    impl FakeRoot {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("vega-gpu-bench-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            FakeRoot(dir)
        }

        fn file(&self, path: &str, content: &str) {
            let path = self.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        fn link(&self, path: &str, target: &str) {
            let path = self.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(target, path).unwrap();
        }

        fn process(&self, pid: u32, exe: &str, comm: &str, fds: &[&str]) {
            self.link(&format!("proc/{pid}/exe"), exe);
            self.file(&format!("proc/{pid}/comm"), &format!("{comm}\n"));
            std::fs::create_dir_all(self.0.join(format!("proc/{pid}/fd"))).unwrap();
            for (fd, target) in fds.iter().enumerate() {
                self.link(&format!("proc/{pid}/fd/{fd}"), target);
            }
        }
    }

    impl Drop for FakeRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_the_rtd3_state_of_one_gpu() {
        let root = FakeRoot::new("read");
        root.file(
            "proc/driver/nvidia/gpus/0000:01:00.0/power",
            &with_first_line("Disabled by default"),
        );
        assert_eq!(read_rtd3(&root.0, "0000:01:00.0"), Rtd3::Disabled);
        assert_eq!(read_rtd3(&root.0, "0000:02:00.0"), Rtd3::Unknown);
    }

    #[test]
    fn lists_only_unexpected_processes_holding_this_gpu() {
        let root = FakeRoot::new("holders");
        let nvidia = "sys/bus/pci/devices/0000:01:00.0";
        let intel = "sys/bus/pci/devices/0000:00:02.0";
        for (device, nodes) in [
            (nvidia, ["card1", "renderD129", "controlD65"]),
            (intel, ["card0", "renderD128", "controlD64"]),
        ] {
            for node in nodes {
                std::fs::create_dir_all(root.0.join(device).join("drm").join(node)).unwrap();
            }
        }
        root.process(
            4100,
            "/usr/bin/blender",
            "blender",
            &["/dev/null", "/dev/nvidia0"],
        );
        root.process(
            3900,
            "/usr/bin/glxgears",
            "glxgears",
            &["/dev/dri/renderD129"],
        );
        root.process(
            2178,
            "/usr/bin/gnome-shell",
            "gnome-shell",
            &["/dev/nvidiactl", "/dev/dri/card1"],
        );
        root.process(
            1200,
            "/usr/bin/nvidia-persistenced",
            "nvidia-persiste",
            &["/dev/nvidiactl", "/dev/nvidia0"],
        );
        root.process(
            5000,
            "/usr/bin/firefox",
            "firefox",
            &["/dev/dri/renderD128"],
        );
        root.process(6000, "/usr/bin/vega", "vega", &["/dev/nvidiactl"]);
        root.process(
            6100,
            "/usr/bin/cuda-app (deleted)",
            "cuda-app",
            &["/dev/nvidia-uvm"],
        );
        // Um processo de outro usuário: o fd não pode ser lido.
        root.file("proc/7000/comm", "secret\n");
        // Entradas que não são processos.
        root.file("proc/self-not-a-pid", "");

        assert_eq!(
            gpu_holders(&root.0, "0000:01:00.0", 6000),
            vec![
                (3900, "glxgears".to_owned()),
                (4100, "blender".to_owned()),
                (6100, "cuda-app".to_owned()),
            ]
        );
    }

    #[test]
    fn the_holder_list_is_capped() {
        let root = FakeRoot::new("cap");
        for pid in 100..130 {
            root.process(pid, "/usr/bin/app", "app", &["/dev/nvidia0"]);
        }
        let holders = gpu_holders(&root.0, "0000:01:00.0", 1);
        assert_eq!(holders.len(), MAX_HOLDERS);
        assert_eq!(holders[0].0, 100);
    }

    #[test]
    fn verdict_rule_for_every_rtd3_state() {
        let sysfs = "/sys/bus/pci/devices/0000:01:00.0";
        let holders = vec![(3900, "glxgears".to_owned()), (4100, "blender".to_owned())];
        let single = |state, suspended, holders: &[(u32, String)]| {
            let found = rtd3_findings(state, suspended, holders, sysfs);
            assert!(found.len() <= 1);
            found.into_iter().next()
        };

        assert!(single(Rtd3::Fine, true, &[]).is_none());

        let awake = single(Rtd3::Fine, false, &holders).unwrap();
        assert_eq!(awake["code"], "gpu.rtd3-awake");
        assert_eq!(awake["severity"], "attention");
        assert_eq!(awake["raw"], "3900 glxgears\n4100 blender");
        assert!(
            awake["text"]
                .as_str()
                .unwrap()
                .contains("programas listados")
        );

        let awake_alone = single(Rtd3::Fine, false, &[]).unwrap();
        assert_eq!(awake_alone["severity"], "attention");
        assert_eq!(awake_alone["raw"], sysfs);
        assert!(
            awake_alone["text"]
                .as_str()
                .unwrap()
                .contains("sem nenhum programa")
        );

        for (state, code) in [
            (Rtd3::Coarse, "gpu.rtd3-coarse"),
            (Rtd3::Unsupported, "gpu.rtd3-unsupported"),
            (Rtd3::Disabled, "gpu.rtd3-disabled"),
            (Rtd3::Unknown, "gpu.rtd3-unknown"),
        ] {
            for suspended in [false, true] {
                let found = single(state, suspended, &[]).unwrap();
                assert_eq!(found["code"], code);
                assert_eq!(found["severity"], "info", "{code}");
                assert_eq!(found["raw"], sysfs);
            }
        }
        let coarse = single(Rtd3::Coarse, false, &holders).unwrap();
        assert_eq!(coarse["raw"], "3900 glxgears\n4100 blender");
    }

    #[test]
    fn the_nitro_without_driver_rtd3_ends_ok_without_waiting() {
        let root = FakeRoot::new("nitro");
        let device = "sys/bus/pci/devices/0000:01:00.0";
        root.file(&format!("{device}/vendor"), "0x10de\n");
        root.file(&format!("{device}/class"), "0x030000\n");
        root.file(&format!("{device}/power/control"), "auto\n");
        root.file(&format!("{device}/power/runtime_status"), "active\n");
        root.link(
            &format!("{device}/driver"),
            "../../../bus/pci/drivers/nvidia",
        );
        root.file(
            "proc/driver/nvidia/gpus/0000:01:00.0/power",
            &with_first_line("Disabled by default"),
        );
        let mut findings = Vec::new();
        let mut metrics = json!({});
        let start = Instant::now();
        nvidia_power_check(&root.0, &|_, _| {}, &mut findings, &mut metrics);
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "não deve esperar 30 s"
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["code"], "gpu.rtd3-disabled");
        assert_eq!(findings[0]["severity"], "info");
        assert_eq!(metrics["nvidiaRTD3"], "disabled");
        assert_eq!(metrics["nvidiaRuntimeSuspended"], false);
    }

    #[test]
    fn several_gpus_report_metrics_by_pci_address() {
        let mut values = serde_json::Map::new();
        values.insert("0000:01:00.0".into(), json!("fine"));
        assert_eq!(per_device(values.clone()), json!("fine"));
        values.insert("0000:02:00.0".into(), json!("unsupported"));
        assert_eq!(
            per_device(values),
            json!({"0000:01:00.0": "fine", "0000:02:00.0": "unsupported"})
        );
    }

    /// O corpo de uma função deste arquivo, até a próxima função de topo.
    fn function_body(source: &str, signature: &str) -> String {
        let start = source.find(signature).expect(signature);
        let rest = &source[start + signature.len()..];
        let end = ["\nfn ", "\npub fn ", "\n#[", "\nconst ", "\nstatic "]
            .iter()
            .filter_map(|marker| rest.find(marker))
            .min()
            .unwrap_or(rest.len());
        rest[..end].to_owned()
    }

    #[test]
    fn the_power_check_never_wakes_the_gpu() {
        let source = include_str!("gpu_bench.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        // Montado em partes para este teste não se acusar.
        let smi = ["nvidia", "-smi"].concat();
        let nvml = ["nv", "ml"].concat();
        assert!(
            !code.contains(&smi),
            "o teste de GPU não pode chamar o {smi}"
        );
        assert!(!code.to_lowercase().contains(&nvml));
        // Nada que rode programas ou abra dispositivos entre o fim do teste
        // e a leitura do runtime_status.
        for function in [
            "fn nvidia_power_check(",
            "fn nvidia_runtime_pm(",
            "fn pci_driver(",
            "pub fn read_rtd3(",
            "pub fn gpu_holders(",
            "fn process_name(",
            "fn runtime_suspended(",
            "fn wait_suspended(",
        ] {
            let body = function_body(code, function);
            for forbidden in ["run(", "Command", "File::open", "OpenOptions"] {
                assert!(!body.contains(forbidden), "{function} usa {forbidden}");
            }
        }
        // E a verificação vem depois do último programa do teste.
        let bench = function_body(code, "fn bench(");
        let check = bench.find("nvidia_power_check(").unwrap();
        assert!(!bench[check..].contains("run("));
    }
}
