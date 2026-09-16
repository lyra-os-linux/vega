//! Optional NVIDIA integration. Startup/refresh use public queries only.
use crate::{i18n::gettext, ui::NvidiaCard};
use adw::prelude::*;
use gtk::glib;
use lyra_vega_dbus::{MetadataClient, SoftwareClient, SoftwareEvent, VegaDbus};

pub fn configure(card: &NvidiaCard, window: &adw::ApplicationWindow, dbus: VegaDbus) {
    let card_refresh = card.clone();
    let bus_refresh = dbus.clone();
    card.refresh.connect_clicked(move |_| {
        if card_refresh.busy.get() {
            return;
        }
        card_refresh.set_busy(true);
        let card = card_refresh.clone();
        let dbus = bus_refresh.clone();
        glib::MainContext::default().spawn_local(async move {
            refresh(&card, &dbus).await;
            card.set_busy(false);
        });
    });
    let card_install = card.clone();
    let bus_install = dbus.clone();
    let parent = window.downgrade();
    card.install.connect_clicked(move |_| {
        if card_install.busy.get() || !card_install.available.get() {
            return;
        }
        let Some(parent) = parent.upgrade() else {
            return;
        };
        card_install.set_busy(true);
        let card = card_install.clone();
        let dbus = bus_install.clone();
        glib::MainContext::default().spawn_local(async move {
            let dialog = installation_dialog();
            // This reviews new repositories/trust/recovery, so it is mandatory
            // even if optional confirmations have been disabled in preferences.
            if dialog.choose_future(Some(&parent)).await != "install" {
                card.set_busy(false);
                return;
            }
            let client = dbus.software();
            let result: Result<(), String> = async {
                let mut events = client.subscribe().await.map_err(|e| e.to_string())?;
                let id = client
                    .install_nvidia(true)
                    .await
                    .map_err(|e| e.to_string())?;
                card.transaction(0);
                loop {
                    // Includes daemon-owner loss and the shared finite watchdog.
                    match events
                        .next_transaction(id)
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        SoftwareEvent::Progress(event) => card.transaction(event.percent),
                        SoftwareEvent::Finished(event) => {
                            return if event.success {
                                Ok(())
                            } else {
                                Err(event.message)
                            };
                        }
                        _ => {}
                    }
                }
            }
            .await;
            // A failure can leave a partial RPM transaction; reread the facts.
            refresh(&card, &dbus).await;
            if let Err(error) = result {
                card.transaction_failed(&error);
            }
            card.set_busy(false);
        });
    });
    let card = card.clone();
    card.set_busy(true);
    glib::MainContext::default().spawn_local(async move {
        refresh(&card, &dbus).await;
        card.set_busy(false);
    });
}

async fn refresh(card: &NvidiaCard, dbus: &VegaDbus) {
    match dbus.metadata().metadata().await {
        Ok(metadata)
            if metadata
                .capabilities
                .iter()
                .any(|c| c == "nvidia-official-v1") => {}
        Ok(_) => {
            card.unavailable(&gettext(
                "Atualize o vegad para usar a integração NVIDIA atual.",
            ));
            return;
        }
        Err(error) => {
            card.unavailable(&error.to_string());
            return;
        }
    }
    match dbus.software().nvidia_status().await {
        Ok(status) => card.show_status(&status, true),
        Err(error) => card.unavailable(&error.to_string()),
    }
}

fn installation_dialog() -> adw::AlertDialog {
    let dialog = adw::AlertDialog::new(
        Some(&gettext("Instalar a integração NVIDIA?")),
        Some(&gettext(
            "O Vega usará os repositórios Lyra no OBS, NVIDIA oficial e openSUSE Leap 16.1. Serão instalados apenas os RPMs qualificados, com verificação de assinaturas e módulo assinado pela SUSE.\n\nÉ necessário criar um snapshot de recuperação. O Vega não remove drivers conflitantes nem troca o kernel automaticamente. Uma falha pode exigir recuperação pelo snapshot. Reinicie somente após uma instalação concluída e verificada.\n\nA autorização de administrador será solicitada ao continuar.",
        )),
    );
    dialog.add_responses(&[
        ("cancel", &gettext("Cancelar")),
        ("install", &gettext("Continuar")),
    ]);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
    dialog
}
