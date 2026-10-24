//! Notificações da sessão (org.freedesktop.Notifications).
//!
//! Substitui o GNotification do vega-gtk: o mesmo id lógico ("updates",
//! "service-failures", "backup-finished") reaproveita a notificação anterior
//! em vez de empilhar várias.

use std::collections::HashMap;
use std::sync::Mutex;

use zbus::zvariant::Value;

static REPLACES: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

pub async fn send(id: &str, summary: &str, body: &str) {
    let Ok(connection) = zbus::Connection::session().await else {
        return;
    };
    let replaces = REPLACES
        .lock()
        .ok()
        .and_then(|map| map.as_ref().and_then(|m| m.get(id).copied()))
        .unwrap_or(0);
    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
    hints.insert("desktop-entry", Value::from("vega"));
    let reply = connection
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                "Vega",
                replaces,
                "vega",
                summary,
                body,
                Vec::<&str>::new(),
                hints,
                -1i32,
            ),
        )
        .await;
    if let Ok(message) = reply
        && let Ok(new_id) = message.body().deserialize::<u32>()
        && let Ok(mut map) = REPLACES.lock()
    {
        map.get_or_insert_with(HashMap::new)
            .insert(id.to_owned(), new_id);
    }
}

#[tauri::command]
pub async fn notify(id: String, summary: String, body: String) {
    send(&id, &summary, &body).await;
}
