use crate::i18n::gettext;
use adw::prelude::*;
use lyra_vega_dbus::NvidiaStatus;
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct NvidiaCard {
    pub root: adw::PreferencesGroup,
    pub install: gtk::Button,
    pub refresh: gtk::Button,
    pub busy: Rc<Cell<bool>>,
    pub available: Rc<Cell<bool>>,
    status: gtk::Label,
    detail: gtk::Label,
    security: gtk::Label,
    recovery: gtk::Label,
    progress: gtk::ProgressBar,
}

impl NvidiaCard {
    pub fn new() -> Self {
        let root = adw::PreferencesGroup::builder()
            .title(gettext("NVIDIA"))
            .description(gettext(
                "Driver oficial NVIDIA com módulo assinado pela SUSE",
            ))
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let label = |text: &str| {
            gtk::Label::builder()
                .label(text)
                .wrap(true)
                .xalign(0.0)
                .build()
        };
        let status = label(&gettext("Carregando…"));
        let detail = label("");
        detail.set_selectable(true);
        detail.add_css_class("dim-label");
        let security = label("");
        let recovery = label("");
        let progress = gtk::ProgressBar::new();
        progress.set_visible(false);
        let install = gtk::Button::with_label(&gettext("Instalar driver NVIDIA"));
        install.add_css_class("suggested-action");
        install.set_sensitive(false);
        let refresh = gtk::Button::with_label(&gettext("Atualizar diagnóstico"));
        // FlowBox wraps at narrow window sizes instead of widening the page.
        let actions = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .min_children_per_line(1)
            .max_children_per_line(2)
            .column_spacing(8)
            .row_spacing(8)
            .build();
        actions.insert(&refresh, -1);
        actions.insert(&install, -1);
        for child in [&status, &security, &recovery, &detail] {
            content.append(child);
        }
        content.append(&progress);
        content.append(&actions);
        root.add(&content);
        Self {
            root,
            install,
            refresh,
            busy: Rc::new(Cell::new(false)),
            available: Rc::new(Cell::new(false)),
            status,
            detail,
            security,
            recovery,
            progress,
        }
    }

    pub fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.refresh.set_sensitive(!busy);
        self.install.set_sensitive(!busy && self.available.get());
    }

    pub fn show_status(&self, status: &NvidiaStatus, capable: bool) {
        self.status.set_label(&state_message(&status.state));
        self.detail.set_label(&status.detail);
        let secure = match status.secure_boot.as_str() {
            "enabled" => gettext("Habilitado"),
            "disabled" => gettext("Desabilitado"),
            "not-applicable" => gettext("Não se aplica (BIOS)"),
            _ => gettext("Desconhecido"),
        };
        self.security
            .set_label(&gettext("Secure Boot: {state}").replace("{state}", &secure));
        self.recovery.set_label(&if status.recovery_snapshot > 0 {
            gettext("Snapshot de recuperação: {id}")
                .replace("{id}", &status.recovery_snapshot.to_string())
        } else {
            String::new()
        });
        self.install.set_label(&if status.state == "unmanaged" {
            gettext("Ativar integração Lyra")
        } else {
            gettext("Instalar driver NVIDIA")
        });
        self.available.set(can_install(status, capable));
        self.install
            .set_visible(capable && matches!(status.state.as_str(), "available" | "unmanaged"));
        self.install
            .set_sensitive(!self.busy.get() && self.available.get());
        self.progress.set_visible(false);
    }

    pub fn unavailable(&self, detail: &str) {
        self.available.set(false);
        self.install.set_sensitive(false);
        self.status
            .set_label(&gettext("Diagnóstico NVIDIA indisponível"));
        self.detail.set_label(detail);
        self.progress.set_visible(false);
    }

    pub fn transaction(&self, percent: u32) {
        self.status
            .set_label(&gettext("Instalando a integração NVIDIA…"));
        self.progress.set_visible(true);
        self.progress
            .set_fraction(f64::from(percent.min(100)) / 100.0);
    }

    pub fn transaction_failed(&self, detail: &str) {
        self.status.set_label(&gettext(
            "A instalação não foi concluída. Consulte os detalhes antes de reiniciar.",
        ));
        self.detail.set_label(detail);
        self.progress.set_visible(false);
    }
}

pub(crate) fn can_install(status: &NvidiaStatus, capable: bool) -> bool {
    capable && status.supported && matches!(status.state.as_str(), "available" | "unmanaged")
}

fn state_message(state: &str) -> String {
    match state {
        "no-gpu" => gettext("Nenhuma GPU NVIDIA detectada."),
        "unsupported-gpu" => {
            gettext("Esta GPU não está na lista de suporte do driver NVIDIA qualificado.")
        }
        "unsupported-system" => {
            gettext("Esta integração requer Lyra OS ou openSUSE Leap 16.1, em x86_64.")
        }
        "unknown-secure-boot" => {
            gettext("Não foi possível verificar o Secure Boot. A instalação está bloqueada.")
        }
        "unknown-boot-kernel" => gettext(
            "Não foi possível identificar o kernel padrão. Revise a configuração de inicialização.",
        ),
        "conflict" => gettext(
            "Há outro driver NVIDIA instalado. A migração precisa ser revisada antes de continuar.",
        ),
        "inconsistent" => gettext(
            "Os pacotes NVIDIA estão incompletos ou com versões incompatíveis. Revise a instalação antes de reiniciar.",
        ),
        "kernel-missing" => gettext(
            "O módulo NVIDIA correspondente não está disponível para o kernel em uso ou para o kernel padrão.",
        ),
        "unsigned-module" => {
            gettext("O módulo encontrado não corresponde ao pacote assinado pela SUSE.")
        }
        "available" => {
            gettext("O driver NVIDIA qualificado está disponível para instalação opcional.")
        }
        "unmanaged" => gettext(
            "Driver oficial instalado. Ative a integração Lyra para manter bibliotecas e módulo em versões correspondentes.",
        ),
        "reboot-required" => {
            gettext("Driver instalado. Reinicie para carregar o módulo e conferir o funcionamento.")
        }
        "active" => {
            gettext("Driver NVIDIA ativo; bibliotecas, módulo e integração Lyra correspondem.")
        }
        _ => gettext("Atualize o vegad para usar a integração NVIDIA atual."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_requires_capability_and_an_explicit_ready_state() {
        let mut status = NvidiaStatus {
            supported: true,
            installed: false,
            reboot_required: false,
            gpu: String::new(),
            secure_boot: "enabled".into(),
            state: "available".into(),
            detail: String::new(),
            recovery_snapshot: 0,
        };
        assert!(can_install(&status, true));
        assert!(!can_install(&status, false));
        for state in [
            "active",
            "unavailable",
            "conflict",
            "inconsistent",
            "kernel-missing",
            "unknown-secure-boot",
            "future-state",
        ] {
            status.state = state.into();
            assert!(!can_install(&status, true), "{state}");
        }
        status.state = "unmanaged".into();
        status.installed = true;
        assert!(can_install(&status, true));
        status.supported = false;
        assert!(!can_install(&status, true));
    }
}
