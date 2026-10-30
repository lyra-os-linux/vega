#[path = "../i18n.rs"]
#[allow(dead_code)]
mod i18n;
use gio::prelude::*;

use i18n::gettext;
use lyra_vega_dbus::{
    DiagnosticsClient, DiagnosticsEvent, PeriodicRun, SoftwareClient, SoftwareEvent, VegaDbus,
};
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};

const APPLICATION_ID: &str = "org.lyraos.Vega.UpdateNotifier";

fn main() -> glib::ExitCode {
    let preferences = read_preferences();
    i18n::init(
        preferences
            .get("language")
            .and_then(Value::as_str)
            .unwrap_or("system"),
    );
    let app = gio::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    let open = gio::SimpleAction::new("open-updates", None);
    open.connect_activate(|_, _| {
        // O argumento chega também a uma janela já aberta (instância única);
        // a variável mantém compatibilidade com o vega-gtk anterior.
        let _ = Command::new("/usr/bin/vega-gtk")
            .arg("--open-updates")
            .env("VEGA_START_PAGE", "software")
            .spawn();
    });
    app.add_action(&open);

    let open_health = gio::SimpleAction::new("open-health", None);
    open_health.connect_activate(|_, _| {
        let _ = Command::new("/usr/bin/vega-gtk")
            .arg("--open-health")
            .spawn();
    });
    app.add_action(&open_health);

    app.connect_activate(|app| {
        let hold = app.hold();
        let app = app.clone();
        glib::MainContext::default().spawn_local(async move {
            let _hold = hold;
            // Verificação periódica (add-periodic-checks): avisa ao entrar na
            // sessão o que mudou enquanto o usuário estava fora, e ao vivo
            // quando o timer termina com novidades.
            let health_app = app.clone();
            glib::MainContext::default().spawn_local(async move {
                loop {
                    if let Ok(dbus) = VegaDbus::connect().await {
                        let client = dbus.diagnostics();
                        if let Ok(mut events) = client.subscribe().await {
                            if let Ok(run) = client.last_periodic().await {
                                notify_health(&health_app, &run);
                            }
                            loop {
                                match events.next().await {
                                    Ok(DiagnosticsEvent::Novelties { .. }) => {
                                        if let Ok(run) = client.last_periodic().await {
                                            notify_health(&health_app, &run);
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                    glib::timeout_future_seconds(300).await;
                }
            });
            let polling_app = app.clone();
            glib::MainContext::default().spawn_local(async move {
                loop {
                    glib::timeout_future_seconds(60).await;
                    if let Ok(dbus) = VegaDbus::connect().await
                        && let Ok(status) = dbus.software().update_status().await
                    {
                        notify_if_needed(&polling_app, status.total_count);
                    }
                }
            });
            loop {
                if let Ok(dbus) = VegaDbus::connect().await {
                    let client = dbus.software();
                    if let Ok(mut events) = client.subscribe().await {
                        if let Ok(status) = client.update_status().await {
                            notify_if_needed(&app, status.total_count);
                        }
                        loop {
                            match events.next().await {
                                Ok(SoftwareEvent::UpdatesAvailable(count)) => {
                                    notify_if_needed(&app, count)
                                }
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        }
                    }
                }
                // Reconnect passive notifications without replaying a transaction.
                glib::timeout_future_seconds(60).await;
            }
        });
    });
    app.run()
}

fn notify_if_needed(app: &gio::Application, count: u32) {
    let state = notification_state_path();
    let previous = fs::read_to_string(&state)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    if count == 0 {
        save_notified_count(&state, count);
        app.withdraw_notification("updates-available");
        return;
    }
    if !update_notifications_enabled() {
        app.withdraw_notification("updates-available");
        return;
    }
    if previous == Some(count) {
        return;
    }
    save_notified_count(&state, count);

    let notification = gio::Notification::new(&gettext("Atualizações disponíveis"));
    notification.set_body(Some(
        &gettext("{count} pacote(s) pendente(s)").replace("{count}", &count.to_string()),
    ));
    notification.set_icon(&gio::ThemedIcon::new("lyra-updates-symbolic"));
    notification.set_default_action("app.open-updates");
    app.send_notification(Some("updates-available"), &notification);
}

/// Uma notificação por execução periódica com novidades.
fn notify_health(app: &gio::Application, run: &PeriodicRun) {
    if run.job_id == 0 || run.novelties == 0 || !health_notifications_enabled() {
        return;
    }
    let state = glib::user_data_dir().join("vega-gtk/health-notifier-job");
    let previous = fs::read_to_string(&state)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    if previous == Some(run.job_id) {
        return;
    }
    save_notified_count(&state, run.job_id);
    let notification = gio::Notification::new(
        &gettext("O Vega encontrou {count} novidade(s) na saúde do sistema")
            .replace("{count}", &run.novelties.to_string()),
    );
    let mut body: Vec<&str> = run
        .novelty_texts
        .iter()
        .take(2)
        .map(String::as_str)
        .collect();
    if run.novelty_texts.len() > 2 {
        body.push("…");
    }
    notification.set_body(Some(&body.join("\n")));
    notification.set_icon(&gio::ThemedIcon::new("dialog-warning-symbolic"));
    notification.set_default_action("app.open-health");
    app.send_notification(Some("health-novelties"), &notification);
}

fn health_notifications_enabled() -> bool {
    read_preferences()
        .get("notify_health")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn save_notified_count(path: &std::path::Path, count: u32) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, format!("{count}\n"));
}

fn notification_state_path() -> PathBuf {
    glib::user_data_dir().join("vega-gtk/update-notifier-count")
}

fn update_notifications_enabled() -> bool {
    read_preferences()
        .get("notify_updates")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn read_preferences() -> Value {
    let path = glib::user_config_dir().join("vega-gtk/preferences.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null)
}
