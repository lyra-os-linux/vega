use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use adw::prelude::*;
use gtk::glib;
use lyra_vega_dbus::{
    PreparationClient, PreparationKey, PreparationStatus, SoftwareEvent, VegaDbus,
};

use crate::i18n::gettext;

struct Inner {
    root: gtk::Box,
    title: gtk::Label,
    detail: gtk::Label,
    feedback: gtk::Label,
    retry: gtk::Button,
    review: gtk::Button,
    key: RefCell<Option<PreparationKey>>,
    busy: Cell<bool>,
    can_retry: Cell<bool>,
}

#[derive(Clone)]
pub struct PreparationPanel(Rc<Inner>);

impl PreparationPanel {
    pub fn new() -> Self {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .visible(false)
            .build();
        root.add_css_class("card");
        root.set_halign(gtk::Align::Fill);
        let title = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["heading"])
            .build();
        let detail = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .build();
        let feedback = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .build();
        let retry = gtk::Button::builder()
            .label(gettext("Tentar novamente"))
            .halign(gtk::Align::Start)
            .visible(false)
            .build();
        let review = gtk::Button::builder()
            .label(gettext("Revisar chave…"))
            .halign(gtk::Align::Start)
            .visible(false)
            .build();
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        content.append(&title);
        content.append(&detail);
        content.append(&feedback);
        content.append(&retry);
        content.append(&review);
        root.append(&content);
        Self(Rc::new(Inner {
            root,
            title,
            detail,
            feedback,
            retry,
            review,
            key: RefCell::new(None),
            busy: Cell::new(false),
            can_retry: Cell::new(false),
        }))
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.0.root
    }

    /// One timer per page, weak ownership and one request in flight. Mapping
    /// refreshes immediately; hidden pages do not send periodic bus requests.
    pub fn connect(&self, page: &gtk::Widget, dbus: VegaDbus) {
        let weak = Rc::downgrade(&self.0);
        let weak_page = page.downgrade();
        let poll_dbus = dbus.clone();
        let poll: Rc<dyn Fn()> = Rc::new(move || {
            if !weak_page.upgrade().is_some_and(|page| page.is_mapped()) {
                return;
            }
            let Some(panel) = weak.upgrade() else {
                return;
            };
            if panel.busy.replace(true) {
                return;
            }
            panel.retry.set_sensitive(false);
            panel.review.set_sensitive(false);
            let client = poll_dbus.preparation();
            glib::MainContext::default().spawn_local(async move {
                let result=client.status().await;
                let keys = if matches!(&result, Ok(Some(status)) if status.state == "awaiting-approval") {
                    client.pending_keys().await
                } else { Ok(Vec::new()) };
                match keys {
                    Ok(keys) => { *panel.key.borrow_mut() = keys.into_iter().next(); },
                    Err(_) => {
                        *panel.key.borrow_mut() = None;
                        panel.feedback.set_visible(true);
                        panel.feedback.set_text(&gettext("Não foi possível carregar a chave pendente. A consulta será repetida."));
                    }
                }
                panel.review.set_visible(panel.key.borrow().is_some());
                panel.review.set_sensitive(true);
                panel.busy.set(false);
                match result {
                    Ok(Some(status)) => panel.show_status(&status),
                    Ok(None) => { panel.root.set_visible(false); panel.can_retry.set(false); },
                    Err(_) => {
                        panel.root.set_visible(true);
                        panel.title.set_text(&gettext("Estado da preparação indisponível"));
                        panel.detail.set_text(&gettext("Não foi possível consultar o serviço. A conexão será verificada novamente."));
                        panel.can_retry.set(false);
                        panel.retry.set_visible(false);
                    }
                }
            });
        });
        let on_map = poll.clone();
        page.connect_map(move |_| on_map());
        let timer_poll = poll.clone();
        let weak_page = page.downgrade();
        glib::timeout_add_local(Duration::from_secs(5), move || {
            if weak_page.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            timer_poll();
            glib::ControlFlow::Continue
        });
        poll();
        let weak = Rc::downgrade(&self.0);
        let review_dbus = dbus.clone();
        let review_poll = poll.clone();
        let weak_page = page.downgrade();
        self.0.review.connect_clicked(move |_| {
            let Some(panel) = weak.upgrade() else {
                return;
            };
            if panel.busy.get() {
                return;
            }
            let Some(key) = panel.key.borrow().clone() else {
                return;
            };
            let Some(parent) = weak_page.upgrade() else {
                return;
            };
            panel.busy.set(true);
            panel.retry.set_sensitive(false);
            panel.review.set_sensitive(false);
            let dbus = review_dbus.clone();
            let refresh = review_poll.clone();
            glib::MainContext::default().spawn_local(async move {
                let dialog = key_review_dialog(&key);
                if dialog.choose_future(Some(&parent)).await == "confirm" {
                    panel.feedback.set_visible(true);
                    panel
                        .feedback
                        .set_text(&gettext("Solicitando autorização…"));
                    let result = approve_key_transaction(&dbus, &key, &panel.feedback).await;
                    match result {
                        Ok(()) => panel
                            .feedback
                            .set_text(&gettext("Chave aprovada. Acompanhando a preparação…")),
                        Err(error) => panel.feedback.set_text(
                            &gettext("Não foi possível aprovar a chave: {detail}")
                                .replace("{detail}", &error),
                        ),
                    }
                }
                panel.busy.set(false);
                panel.review.set_sensitive(true);
                panel.retry.set_sensitive(panel.can_retry.get());
                refresh();
            });
        });
        let weak = Rc::downgrade(&self.0);
        self.0.retry.connect_clicked(move |_| {
            let Some(panel) = weak.upgrade() else {
                return;
            };
            if !panel.can_retry.get() || panel.busy.replace(true) {
                return;
            }
            panel.retry.set_sensitive(false);
            panel.review.set_sensitive(false);
            panel.feedback.set_visible(true);
            panel
                .feedback
                .set_text(&gettext("Solicitando autorização…"));
            let client = dbus.preparation();
            let refresh = poll.clone();
            glib::MainContext::default().spawn_local(async move {
                let result = client.retry().await;
                panel.busy.set(false);
                match result {
                    Ok(()) => {
                        panel.feedback.set_text(&gettext(
                            "Nova tentativa solicitada. Acompanhando o serviço…",
                        ));
                        // Do not permit a second click before observing the new state.
                        panel.can_retry.set(false);
                    }
                    Err(error) => {
                        panel.feedback.set_text(
                            &gettext("Não foi possível solicitar uma nova tentativa: {detail}")
                                .replace("{detail}", &error.to_string()),
                        );
                        panel.retry.set_sensitive(panel.can_retry.get());
                    }
                }
                refresh();
            });
        });
    }
}

fn key_review_dialog(key: &PreparationKey) -> adw::AlertDialog {
    let body = gettext("Repositório: {repo}\nAssinante: {user}\nImpressão digital completa: {fingerprint}\n\nConfira a impressão digital com o responsável pelo repositório antes de confiar. A aprovação retomará a preparação.")
        .replace("{repo}", &key.repo).replace("{user}", &key.user_id).replace("{fingerprint}", &key.fingerprint);
    let dialog = adw::AlertDialog::new(
        Some(&gettext("Confiar na chave do repositório?")),
        Some(&body),
    );
    dialog.add_responses(&[
        ("cancel", &gettext("Cancelar")),
        ("confirm", &gettext("Confiar")),
    ]);
    dialog.set_body_use_markup(false);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog
}

async fn approve_key_transaction(
    dbus: &VegaDbus,
    key: &PreparationKey,
    feedback: &gtk::Label,
) -> Result<(), String> {
    // Subscribe before starting so fast completion cannot race the listener.
    let mut events = dbus
        .software()
        .subscribe()
        .await
        .map_err(|error| error.to_string())?;
    let id = dbus
        .preparation()
        .approve_key(key)
        .await
        .map_err(|error| error.to_string())?;
    loop {
        match events
            .next_transaction(id)
            .await
            .map_err(|error| error.to_string())?
        {
            SoftwareEvent::Progress(progress) => feedback.set_text(&progress.message),
            SoftwareEvent::Finished(finished) => {
                return if finished.success {
                    Ok(())
                } else {
                    Err(finished.message)
                };
            }
            _ => {}
        }
    }
}

impl Inner {
    fn show_status(&self, status: &PreparationStatus) {
        let Some((title, detail)) = presentation(status) else {
            self.root.set_visible(false);
            self.can_retry.set(false);
            return;
        };
        self.root.set_visible(true);
        self.title.set_text(&title);
        self.detail.set_text(&detail);
        self.can_retry.set(status.can_retry);
        self.retry.set_visible(status.can_retry);
        self.retry
            .set_sensitive(status.can_retry && !self.busy.get());
        if matches!(status.state.as_str(), "running" | "completed") {
            self.feedback.set_visible(false);
        }
    }
}

fn local_time(value: &str) -> String {
    glib::DateTime::from_iso8601(value, None)
        .ok()
        .and_then(|date| date.to_local().ok())
        .and_then(|date| date.format("%x %X").ok())
        .map(|text| text.to_string())
        .unwrap_or_else(|| value.to_owned())
}

fn presentation(status: &PreparationStatus) -> Option<(String, String)> {
    let title = match status.state.as_str() {
        "unavailable" | "skipped" => return None,
        "pending" => gettext("Preparação dos repositórios pendente"),
        "running" => gettext("Preparando os repositórios"),
        "waiting-retry" => gettext("Aguardando nova tentativa"),
        "awaiting-approval" => gettext("Uma chave de assinatura precisa de aprovação"),
        "failed" => gettext("A preparação dos repositórios não terminou"),
        "completed" => gettext("Repositórios preparados"),
        _ => gettext("Estado da preparação indisponível"),
    };
    let mut parts = Vec::new();
    if status.state == "running" {
        parts.push(match status.phase.as_str() {
            "importing-keys" => gettext("Conferindo as chaves de assinatura autorizadas…"),
            "refreshing" => gettext("Atualizando os metadados dos repositórios…"),
            "listing-updates" => gettext("Consultando as atualizações disponíveis…"),
            "publishing" => gettext("Registrando o resultado da preparação…"),
            _ => gettext("Iniciando a preparação…"),
        });
    } else if status.state == "completed" {
        parts.push(gettext(
            "Os repositórios estão prontos. Você pode escolher quais atualizações instalar.",
        ));
    } else if status.state == "pending" {
        parts.push(gettext("A preparação ainda não começou."));
    }
    if !matches!(status.state.as_str(), "running" | "completed") {
        match status.error_kind.as_str() {
            "network"=>parts.push(gettext("Verifique a conexão de rede. Não foi possível acessar os repositórios.")),
            "untrusted-key"=>parts.push(gettext("É necessário revisar e aprovar a chave do repositório. Apenas aguardar ou tentar novamente não autoriza a chave.")),
            "interrupted"=>parts.push(gettext("A preparação foi interrompida. Você pode tentar novamente.")),
            "operation-failed"|"service-failed"=>parts.push(gettext("O serviço não concluiu a preparação. Tente novamente ou consulte o registro do serviço.")),
            _ if !status.last_error.is_empty()=>parts.push(status.last_error.clone()),
            _=>{},
        }
    }
    if !status.updated_at.is_empty() {
        parts.push(
            gettext("Última atualização: {time}")
                .replace("{time}", &local_time(&status.updated_at)),
        );
    }
    if !status.next_retry_at.is_empty() {
        parts.push(
            gettext("Próxima tentativa automática, aproximadamente: {time}")
                .replace("{time}", &local_time(&status.next_retry_at)),
        );
    }
    Some((title, parts.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn status(state: &str, kind: &str) -> PreparationStatus {
        PreparationStatus {
            state: state.into(),
            phase: "refreshing".into(),
            error_kind: kind.into(),
            last_error: "".into(),
            updated_at: "".into(),
            next_retry_at: "".into(),
            can_retry: false,
        }
    }
    #[test]
    fn hides_unavailable_and_exempted_states() {
        assert!(presentation(&status("unavailable", "")).is_none());
        assert!(presentation(&status("skipped", "")).is_none());
    }
    #[test]
    fn distinguishes_action_required_network_and_completion() {
        let key = presentation(&status("awaiting-approval", "untrusted-key")).unwrap();
        let network = presentation(&status("waiting-retry", "network")).unwrap();
        let complete = presentation(&status("completed", "")).unwrap();
        assert_ne!(key, network);
        assert_ne!(network, complete);
        assert!(!key.1.is_empty());
        assert!(!network.1.is_empty());
        assert!(!complete.1.is_empty());
    }
    #[test]
    #[ignore = "requires a GTK display (Broadway is sufficient)"]
    fn renders_status_and_retry_controls() {
        adw::init().unwrap();
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(false);
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        let panel = PreparationPanel::new();
        let window = gtk::Window::builder()
            .default_width(720)
            .default_height(280)
            .child(panel.widget())
            .build();
        let key = PreparationKey {
            repo: "fixture <untrusted>".into(),
            fingerprint: "0123456789ABCDEF0123456789ABCDEF01234567".into(),
            user_id: "Fixture signer".into(),
            token: "review-token".into(),
        };
        let dialog = key_review_dialog(&key);
        assert_eq!(dialog.default_response().as_deref(), Some("cancel"));
        assert_eq!(dialog.close_response(), "cancel");
        assert!(!dialog.is_body_use_markup());
        assert!(dialog.body().contains(&key.fingerprint));
        assert!(dialog.body().contains(&key.repo));
        window.present();
        for (state, kind, retry) in [
            ("waiting-retry", "network", true),
            ("awaiting-approval", "untrusted-key", true),
            ("running", "", false),
            ("completed", "", false),
        ] {
            let mut value = status(state, kind);
            value.can_retry = retry;
            panel.0.show_status(&value);
            panel.0.review.set_visible(state == "awaiting-approval");
            for _ in 0..100 {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(panel.widget().is_visible());
            assert_eq!(panel.0.retry.is_visible(), retry);
            assert_eq!(panel.0.retry.is_sensitive(), retry);
            assert!(!panel.0.title.text().is_empty());
            assert!(panel.0.title.is_mapped());
            assert!(panel.0.title.width() > 0);
            assert!(panel.0.title.height() > 0);
            if let Some(directory) = std::env::var_os("VEGA_PREPARATION_SCREENSHOTS") {
                panel.widget().allocate(720, 280, -1, None);
                let snapshot = gtk::Snapshot::new();
                window.snapshot_child(panel.widget(), &snapshot);
                let node = snapshot.to_node().unwrap();
                let renderer = gtk::gsk::CairoRenderer::new();
                renderer.realize(None::<&gtk::gdk::Surface>).unwrap();
                let texture = renderer.render_texture(&node, None);
                renderer.unrealize();
                texture
                    .save_to_png(std::path::Path::new(&directory).join(format!("{state}.png")))
                    .unwrap();
            }
        }
        panel.0.show_status(&status("unavailable", ""));
        assert!(!panel.widget().is_visible());
        window.close();
    }
}
