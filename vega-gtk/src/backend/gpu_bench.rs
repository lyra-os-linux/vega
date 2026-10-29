//! Teste de GPU do diagnóstico (vega#165, teste 7). Roda aqui, na sessão do
//! usuário, porque o vegad (root, sem sessão gráfica) não alcança o EGL e o
//! Wayland do desktop. O resultado tem a mesma forma de uma etapa do
//! relatório do vegad (`id`, `verdict`, `summary`, `findings`, `metrics`),
//! para a página juntar tudo num relatório só.

use std::io::Read;
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

/// A NVIDIA com gerenciamento de energia em tempo de execução (RTD3).
fn nvidia_runtime_pm() -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir("/sys/bus/pci/devices").ok()?;
    entries.flatten().map(|entry| entry.path()).find(|path| {
        let read = |name: &str| std::fs::read_to_string(path.join(name)).unwrap_or_default();
        read("vendor").trim() == "0x10de"
            && read("class").starts_with("0x03")
            && read("power/control").trim() == "auto"
    })
}

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
        let out = run(
            "glmark2-es2-wayland",
            &["--off-screen"],
            &env,
            Duration::from_secs(300),
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
    if !cancelled && let Some(device) = nvidia_runtime_pm() {
        progress(92, gettext("Conferindo se a NVIDIA volta a dormir"));
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut suspended = false;
        while Instant::now() < deadline && !CANCELLED.load(Ordering::Relaxed) {
            if std::fs::read_to_string(device.join("power/runtime_status"))
                .is_ok_and(|s| s.trim() == "suspended")
            {
                suspended = true;
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        metrics["nvidiaRuntimeSuspended"] = json!(suspended);
        if !suspended && !CANCELLED.load(Ordering::Relaxed) {
            findings.push(finding(
                "attention",
                "gpu.rtd3-awake",
                gettext("A NVIDIA não voltou a economizar energia 30 s depois do teste; algum programa pode estar usando a GPU"),
                device.display().to_string(),
            ));
        }
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
}
