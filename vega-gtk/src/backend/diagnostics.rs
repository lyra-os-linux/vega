//! Diagnóstico de saúde (vega#165): os testes do vegad pela interface
//! `org.lyraos.Vega1.Diagnostics` e o teste de GPU, que roda aqui, na sessão
//! do usuário (`gpu_bench`).
//!
//! O andamento chega à interface pelo evento `diagnostics-event`. Um job pode
//! continuar no daemon com a janela fechada; ao reabrir, a página chama
//! `diagnostics_current` e `diagnostics_follow` para voltar a acompanhá-lo.

use std::collections::HashSet;
use std::sync::Mutex;

use lyra_vega_dbus::{
    DiagnosticJob, DiagnosticOptions, DiagnosticTest, DiagnosticsClient, DiagnosticsEvent,
    DiagnosticsEventStream,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

use super::{CmdResult, State, dto, err, list};

dto!(Test from DiagnosticTest {
    id: String,
    available: bool,
    reason: String,
    estimated_seconds: u32,
    requires_ac: bool,
});

dto!(Job from DiagnosticJob {
    job_id: u32,
    tests: Vec<String>,
    stage: String,
    percent: u32,
});

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    duration_seconds: Option<u32>,
    memory_percent: Option<u32>,
    boots_back: Option<u32>,
}

/// Eventos para a interface, um tipo por sinal do contrato.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Event {
    Progress {
        job: u32,
        stage: String,
        percent: u32,
        message: String,
    },
    Sample {
        job: u32,
        pkg_temp_c: Option<i32>,
        max_core_temp_c: Option<i32>,
        avg_mhz: Option<u32>,
        throttle_events: Option<u32>,
        nvme_temp_c: Option<i32>,
        load1: Option<f64>,
    },
    Stage {
        job: u32,
        stage: String,
        verdict: String,
        summary: String,
    },
    Finished {
        job: u32,
        result: String,
        message: String,
    },
}

impl Event {
    fn from_signal(event: DiagnosticsEvent) -> Self {
        match event {
            DiagnosticsEvent::Progress(p) => Self::Progress {
                job: p.job_id,
                stage: p.stage,
                percent: p.percent,
                message: p.message,
            },
            DiagnosticsEvent::Sample(s) => Self::Sample {
                job: s.job_id,
                pkg_temp_c: s.pkg_temp_c,
                max_core_temp_c: s.max_core_temp_c,
                avg_mhz: s.avg_mhz,
                throttle_events: s.throttle_events,
                nvme_temp_c: s.nvme_temp_c,
                load1: s.load1,
            },
            DiagnosticsEvent::StageFinished(f) => Self::Stage {
                job: f.job_id,
                stage: f.stage,
                verdict: f.verdict.as_str().to_owned(),
                summary: f.summary,
            },
            DiagnosticsEvent::Finished(f) => Self::Finished {
                job: f.job_id,
                result: f.result.as_str().to_owned(),
                message: f.message,
            },
        }
    }

    fn job(&self) -> u32 {
        match self {
            Self::Progress { job, .. }
            | Self::Sample { job, .. }
            | Self::Stage { job, .. }
            | Self::Finished { job, .. } => *job,
        }
    }
}

/// Jobs que já têm uma tarefa repassando sinais, para a página reaberta
/// não duplicar eventos.
static FOLLOWED: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

fn claim(job: u32) -> bool {
    let mut guard = FOLLOWED.lock().unwrap_or_else(|poison| poison.into_inner());
    guard.get_or_insert_with(HashSet::new).insert(job)
}

fn release(job: u32) {
    let mut guard = FOLLOWED.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(set) = guard.as_mut() {
        set.remove(&job);
    }
}

async fn follow(app: AppHandle, mut events: DiagnosticsEventStream, job: u32) {
    loop {
        match events.next().await {
            Ok(signal) => {
                let event = Event::from_signal(signal);
                if event.job() != job {
                    continue;
                }
                let done = matches!(event, Event::Finished { .. });
                let _ = app.emit("diagnostics-event", event);
                if done {
                    break;
                }
            }
            Err(error) => {
                let _ = app.emit(
                    "diagnostics-event",
                    Event::Finished {
                        job,
                        result: "fail".into(),
                        message: err(error),
                    },
                );
                break;
            }
        }
    }
    release(job);
}

#[tauri::command]
pub async fn diagnostics_tests(state: State<'_>) -> CmdResult<Vec<Test>> {
    state
        .dbus()
        .await?
        .diagnostics()
        .list_tests()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn diagnostics_current(state: State<'_>) -> CmdResult<Job> {
    state
        .dbus()
        .await?
        .diagnostics()
        .current()
        .await
        .map(Into::into)
        .map_err(err)
}

/// Assina os sinais antes de iniciar, para não perder uma etapa rápida.
#[tauri::command]
pub async fn diagnostics_start(
    app: AppHandle,
    state: State<'_>,
    tests: Vec<String>,
    options: Options,
) -> CmdResult<u32> {
    let diagnostics = state.dbus().await?.diagnostics();
    let events = diagnostics.subscribe().await.map_err(err)?;
    let options = DiagnosticOptions {
        duration_seconds: options.duration_seconds,
        memory_percent: options.memory_percent,
        boots_back: options.boots_back,
        locale: Some(crate::i18n::active_locale().to_owned()),
    };
    let job = diagnostics.start(&tests, &options).await.map_err(err)?;
    if claim(job) {
        tauri::async_runtime::spawn(follow(app, events, job));
    }
    Ok(job)
}

/// Volta a acompanhar um job que começou antes desta janela abrir.
#[tauri::command]
pub async fn diagnostics_follow(app: AppHandle, state: State<'_>, job: u32) -> CmdResult<()> {
    if job == 0 || !claim(job) {
        return Ok(());
    }
    let events = match state.dbus().await?.diagnostics().subscribe().await {
        Ok(events) => events,
        Err(error) => {
            release(job);
            return Err(err(error));
        }
    };
    tauri::async_runtime::spawn(follow(app, events, job));
    Ok(())
}

#[tauri::command]
pub async fn diagnostics_cancel(state: State<'_>, job: u32) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .diagnostics()
        .cancel(job)
        .await
        .map_err(err)
}

/// Relatório JSON (schema 1) do job; 0 é o mais recente.
#[tauri::command]
pub async fn diagnostics_report(state: State<'_>, job: u32, redact: bool) -> CmdResult<String> {
    state
        .dbus()
        .await?
        .diagnostics()
        .report(job, redact)
        .await
        .map_err(err)
}

/// Salva o relatório já montado pela interface. O destino vem do diálogo
/// nativo aberto aqui, nunca de um caminho enviado pela página.
#[tauri::command]
pub async fn diagnostics_export(
    app: AppHandle,
    file_name: String,
    content: String,
) -> CmdResult<bool> {
    let name = std::path::Path::new(&file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("vega-diagnostico.txt")
        .to_owned();
    let extension = name.rsplit('.').next().unwrap_or("txt").to_owned();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(&name)
            .add_filter(extension.to_uppercase(), &[extension.as_str()])
            .blocking_save_file()
    })
    .await
    .map_err(err)?;
    let Some(path) = picked else {
        return Ok(false);
    };
    let path = path.into_path().map_err(err)?;
    std::fs::write(&path, content).map_err(err)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_is_followed_once_until_it_finishes() {
        assert!(claim(4242));
        assert!(!claim(4242));
        release(4242);
        assert!(claim(4242));
        release(4242);
    }

    #[test]
    fn events_serialize_with_a_type_tag() {
        let event = Event::Stage {
            job: 3,
            stage: "disk-health".into(),
            verdict: "ok".into(),
            summary: "1 de 1".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "stage");
        assert_eq!(json["job"], 3);
        let sample = Event::Sample {
            job: 3,
            pkg_temp_c: Some(80),
            max_core_temp_c: None,
            avg_mhz: Some(3600),
            throttle_events: None,
            nvme_temp_c: None,
            load1: None,
        };
        let json = serde_json::to_value(&sample).unwrap();
        assert_eq!(json["type"], "sample");
        assert_eq!(json["pkg_temp_c"], 80);
        assert!(json["max_core_temp_c"].is_null());
    }

    #[test]
    fn options_accept_the_page_camel_case() {
        let options: Options =
            serde_json::from_str(r#"{"durationSeconds":60,"memoryPercent":70}"#).unwrap();
        assert_eq!(options.duration_seconds, Some(60));
        assert_eq!(options.memory_percent, Some(70));
        assert_eq!(options.boots_back, None);
    }
}
