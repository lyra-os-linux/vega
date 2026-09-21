use crate::i18n::gettext;
use adw::prelude::*;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use vega_virtualization::{Action, Backend, Connection, CreateRequest, Machine, State};

#[derive(Clone)]
pub struct VirtualizationPage {
    pub root: gtk::Widget,
    connection: gtk::DropDown,
    search: gtk::SearchEntry,
    refresh: gtk::Button,
    create: gtk::Button,
    cancel: gtk::Button,
    status: gtk::Label,
    list: gtk::ListBox,
    busy: Rc<Cell<bool>>,
    poll_enabled: Rc<Cell<bool>>,
    machines: Rc<RefCell<Vec<Machine>>>,
}

impl VirtualizationPage {
    pub fn new() -> Self {
        Self::with_refresh(true)
    }

    fn with_refresh(auto_refresh: bool) -> Self {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        body.set_margin_top(24);
        body.set_margin_bottom(24);
        body.set_margin_start(24);
        body.set_margin_end(24);
        let title = gtk::Label::builder()
            .label(gettext("Máquinas virtuais"))
            .xalign(0.0)
            .css_classes(["title-1"])
            .build();
        body.append(&title);
        let description = gtk::Label::builder()
            .label(gettext("Gerencie aqui e abra a tela da máquina no Lyra VMs. Fechar a janela mantém a máquina em execução."))
            .wrap(true).xalign(0.0).build();
        body.append(&description);
        let connection = gtk::DropDown::from_strings(&[&gettext("Pessoais"), &gettext("Sistema")]);
        connection.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Conexão das máquinas virtuais",
        ))]);
        let refresh = gtk::Button::with_label(&gettext("Atualizar"));
        let create = gtk::Button::with_label(&gettext("Nova máquina"));
        create.add_css_class("suggested-action");
        let cancel = gtk::Button::with_label(&gettext("Cancelar criação"));
        cancel.set_visible(false);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        toolbar.append(&connection);
        toolbar.append(&refresh);
        toolbar.append(&create);
        toolbar.append(&cancel);
        body.append(&toolbar);
        let search = gtk::SearchEntry::builder()
            .placeholder_text(gettext("Buscar máquina virtual…"))
            .build();
        search.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Buscar máquina virtual",
        ))]);
        body.append(&search);
        let status = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .build();
        body.append(&status);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        body.append(&list);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&body)
            .vexpand(true)
            .build();
        let page = Self {
            root: scroll.upcast(),
            connection,
            refresh,
            create,
            cancel,
            search,
            status,
            list,
            busy: Rc::new(Cell::new(false)),
            poll_enabled: Rc::new(Cell::new(true)),
            machines: Rc::new(RefCell::new(Vec::new())),
        };
        let p = page.clone();
        page.refresh.connect_clicked(move |_| p.load());
        let p = page.clone();
        page.create.connect_clicked(move |_| p.create_dialog());
        let p = page.clone();
        page.connection.connect_selected_notify(move |_| {
            p.machines.borrow_mut().clear();
            p.render();
            p.load();
        });
        let p = page.clone();
        page.search.connect_search_changed(move |_| p.render());
        if !auto_refresh {
            return page;
        }
        let timer = Rc::new(RefCell::new(None::<glib::SourceId>));
        let active_timer = timer.clone();
        let p = page.clone();
        page.root.connect_map(move |_| {
            p.load();
            let p = p.clone();
            *active_timer.borrow_mut() = Some(glib::timeout_add_seconds_local(5, move || {
                if p.root.is_mapped() && !p.busy.get() && p.poll_enabled.get() {
                    p.load();
                }
                glib::ControlFlow::Continue
            }));
        });
        page.root.connect_unmap(move |_| {
            if let Some(source) = timer.borrow_mut().take() {
                source.remove();
            }
        });
        page
    }

    fn selected_connection(&self) -> Connection {
        if self.connection.selected() == 1 {
            Connection::System
        } else {
            Connection::Personal
        }
    }

    fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.connection.set_sensitive(!busy);
        self.refresh.set_sensitive(!busy);
        self.create
            .set_sensitive(!busy && self.selected_connection() == Connection::Personal);
        self.list.set_sensitive(!busy);
    }

    fn load(&self) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        if self.machines.borrow().is_empty() {
            self.status
                .set_label(&gettext("Carregando máquinas virtuais…"));
        }
        let connection = self.selected_connection();
        let page = self.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || Backend::open(connection, true)?.list()).await;
            page.set_busy(false);
            match result {
                Ok(Ok(machines)) => {
                    page.poll_enabled.set(true);
                    page.status.set_label("");
                    if *page.machines.borrow() != machines || page.list.first_child().is_none() {
                        *page.machines.borrow_mut() = machines;
                        page.render();
                    }
                }
                Ok(Err(error)) => page.failed(&error),
                Err(_) => page.failed(&gettext("A operação foi interrompida.")),
            }
        });
    }

    fn operation_failed(&self, error: &str) {
        self.poll_enabled.set(false);
        self.status.set_label(error);
    }

    fn failed(&self, error: &str) {
        self.poll_enabled.set(false);
        self.machines.borrow_mut().clear();
        self.render();
        self.status.set_label(&format!("{}\n{error}", gettext("Não foi possível acessar as máquinas virtuais. Confira o serviço libvirt e tente novamente.")));
    }

    fn render(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let query = self.search.text().to_lowercase();
        let machines = self.machines.borrow();
        for machine in machines
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&query))
        {
            let row = adw::ExpanderRow::builder()
                .use_markup(false)
                .subtitle(format!(
                    "{} · {} vCPU · {} MiB",
                    state_label(machine.state),
                    machine.cpus,
                    machine.memory_kib / 1024
                ))
                .build();
            // Construct properties may be applied before use-markup. Set the
            // untrusted title only after markup has definitely been disabled.
            row.set_title(&machine.name);
            let actions = gtk::FlowBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .min_children_per_line(1)
                .max_children_per_line(4)
                .row_spacing(8)
                .column_spacing(8)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(12)
                .margin_end(12)
                .build();
            let console = gtk::Button::with_label(&gettext("Abrir tela"));
            console.set_sensitive(matches!(machine.state, State::Running | State::Paused));
            let uuid = machine.uuid.clone();
            let connection = self.selected_connection();
            let status = self.status.clone();
            console.connect_clicked(move |_| {
                match gio::Subprocess::newv(
                    &[
                        std::ffi::OsStr::new("lyra-vms"),
                        std::ffi::OsStr::new("--connect"),
                        std::ffi::OsStr::new(connection.uri()),
                        std::ffi::OsStr::new("--uuid"),
                        std::ffi::OsStr::new(&uuid),
                    ],
                    gio::SubprocessFlags::NONE,
                ) {
                    Ok(process) => {
                        glib::spawn_future_local(async move {
                            let _ = process.wait_future().await;
                        });
                    }
                    Err(error) => status.set_label(&format!(
                        "{}\n{error}",
                        gettext(
                            "Não foi possível abrir o Lyra VMs. Confira se o pacote está instalado."
                        )
                    )),
                }
            });
            actions.insert(&console, -1);
            if machine.state == State::Off && machine.persistent {
                let configure = gtk::Button::with_label(&gettext("CPU e memória"));
                let page = self.clone();
                let machine = machine.clone();
                configure.connect_clicked(move |_| page.configure_dialog(&machine));
                actions.insert(&configure, -1);
            }
            for (action, label) in [
                (Action::Start, gettext("Iniciar")),
                (Action::Shutdown, gettext("Desligar")),
                (Action::Pause, gettext("Pausar")),
                (Action::Resume, gettext("Retomar")),
                (Action::ForceStop, gettext("Forçar parada")),
                (Action::Remove, gettext("Remover máquina")),
            ] {
                if !action.allowed(machine.state, machine.persistent) {
                    continue;
                }
                let button = gtk::Button::with_label(&label);
                let page = self.clone();
                let machine = machine.clone();
                button.connect_clicked(move |_| page.confirm(&machine, action));
                actions.insert(&button, -1);
            }
            row.add_row(&actions);
            self.list.append(&row);
        }
        if self.list.first_child().is_none() {
            let text = if machines.is_empty() {
                gettext("Nenhuma máquina nesta conexão.")
            } else {
                gettext("Nenhuma máquina corresponde à busca.")
            };
            self.list.append(
                &gtk::Label::builder()
                    .label(text)
                    .margin_top(32)
                    .margin_bottom(32)
                    .build(),
            );
        }
    }

    fn confirm(&self, machine: &Machine, action: Action) {
        if self.busy.get() {
            return;
        }
        if !matches!(
            action,
            Action::Shutdown | Action::ForceStop | Action::Remove
        ) {
            self.execute(machine, action);
            return;
        }
        let message = match action {
            Action::Remove => {
                gettext("Remover a definição desta máquina? Os discos serão preservados.")
            }
            Action::ForceStop => {
                gettext("Interromper imediatamente? Dados não salvos podem ser perdidos.")
            }
            _ => gettext("Solicitar que o sistema convidado desligue?"),
        };
        let dialog = adw::AlertDialog::builder()
            .heading(&machine.name)
            .body(message)
            .build();
        dialog.add_responses(&[
            ("cancel", &gettext("Cancelar")),
            ("proceed", &gettext("Confirmar")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("proceed", adw::ResponseAppearance::Destructive);
        let page = self.clone();
        let machine = machine.clone();
        // Capture the original connection: no action can be redirected by a switch.
        let connection = self.selected_connection();
        dialog.connect_response(None, move |_, response| {
            if response == "proceed" && connection == page.selected_connection() {
                page.execute(&machine, action);
            }
        });
        dialog.present(Some(&self.root));
    }

    fn execute(&self, machine: &Machine, action: Action) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        self.status.set_label(&gettext("Executando operação…"));
        let uuid = machine.uuid.clone();
        let connection = self.selected_connection();
        let page = self.clone();
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || Backend::open(connection, false)?.act(&uuid, action))
                    .await;
            page.set_busy(false);
            match result {
                Ok(Ok(())) => page.load(),
                Ok(Err(error)) => page.operation_failed(&error),
                Err(_) => page
                    .status
                    .set_label(&gettext("A operação foi interrompida.")),
            }
        });
    }

    fn create_dialog(&self) {
        if self.busy.get() || self.selected_connection() != Connection::Personal {
            return;
        }
        let dialog = adw::AlertDialog::builder().heading(gettext("Nova máquina pessoal"))
            .body(gettext("Instalação por ISO, com BIOS e rede compartilhada. Uma cópia da mídia e o disco ficarão na pasta de dados do Lyra VMs."))
            .build();
        let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let name = gtk::Entry::builder()
            .placeholder_text(gettext("Nome da máquina"))
            .build();
        name.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Nome da máquina",
        ))]);
        form.append(&name);
        let cpu = gtk::SpinButton::with_range(1.0, 64.0, 1.0);
        cpu.set_value(2.0);
        let ram = gtk::SpinButton::with_range(512.0, 262144.0, 512.0);
        ram.set_value(2048.0);
        let disk = gtk::SpinButton::with_range(4.0, 2048.0, 1.0);
        disk.set_value(32.0);
        for (text, control) in [
            (gettext("CPUs virtuais"), &cpu),
            (gettext("Memória (MiB)"), &ram),
            (gettext("Disco (GiB)"), &disk),
        ] {
            let row = adw::ActionRow::builder().title(&text).build();
            control.set_valign(gtk::Align::Center);
            control.update_property(&[gtk::accessible::Property::Label(&text)]);
            row.add_suffix(control);
            form.append(&row);
        }
        let media = gtk::Button::with_label(&gettext("Selecionar ISO…"));
        let path = Rc::new(RefCell::new(None));
        let selected = path.clone();
        let root = self.root.clone();
        media.connect_clicked(move |button| {
            let chooser = gtk::FileDialog::builder()
                .title(gettext("Selecionar ISO"))
                .build();
            let parent = root.root().and_downcast::<gtk::Window>();
            let selected = selected.clone();
            let button = button.clone();
            chooser.open(parent.as_ref(), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    button.set_label(&file.basename().unwrap_or_default().to_string_lossy());
                    *selected.borrow_mut() = Some(path);
                }
            });
        });
        form.append(&media);
        dialog.set_extra_child(Some(&form));
        dialog.add_responses(&[
            ("cancel", &gettext("Cancelar")),
            ("create", &gettext("Criar")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
        let page = self.clone();
        dialog.connect_response(None, move |_, response| {
            if response != "create" {
                return;
            }
            let Some(iso) = path.borrow().clone() else {
                page.status
                    .set_label(&gettext("Selecione uma ISO antes de criar a máquina."));
                return;
            };
            let request = CreateRequest {
                name: name.text().to_string(),
                iso,
                cpus: cpu.value_as_int() as u32,
                memory_mib: ram.value_as_int() as u64,
                disk_gib: disk.value_as_int() as u64,
            };
            page.create_machine(request);
        });
        dialog.present(Some(&self.root));
    }

    fn configure_dialog(&self, machine: &Machine) {
        if self.busy.get() {
            return;
        }
        let connection = self.selected_connection();
        let dialog = adw::AlertDialog::builder()
            .heading(&machine.name)
            .body(gettext(
                "CPU e memória serão alteradas com a máquina desligada.",
            ))
            .build();
        let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let cpu = gtk::SpinButton::with_range(1.0, 64.0, 1.0);
        cpu.set_value(f64::from(machine.cpus));
        let ram = gtk::SpinButton::with_range(512.0, 262144.0, 512.0);
        ram.set_value((machine.memory_kib / 1024) as f64);
        for (text, control) in [
            (gettext("CPUs virtuais"), &cpu),
            (gettext("Memória (MiB)"), &ram),
        ] {
            let row = adw::ActionRow::builder().title(&text).build();
            control.update_property(&[gtk::accessible::Property::Label(&text)]);
            control.set_valign(gtk::Align::Center);
            row.add_suffix(control);
            form.append(&row);
        }
        dialog.set_extra_child(Some(&form));
        dialog.add_responses(&[
            ("cancel", &gettext("Cancelar")),
            ("save", &gettext("Salvar")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let page = self.clone();
        let uuid = machine.uuid.clone();
        dialog.connect_response(None, move |_, response| {
            if response != "save" || page.busy.get() || page.selected_connection() != connection {
                return;
            }
            page.set_busy(true);
            page.status.set_label(&gettext("Executando operação…"));
            let uuid = uuid.clone();
            let page = page.clone();
            let cpu = cpu.value_as_int() as u32;
            let ram = ram.value_as_int() as u64;
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || {
                    Backend::open(connection, false)?.configure(&uuid, cpu, ram)
                })
                .await;
                page.set_busy(false);
                match result {
                    Ok(Ok(())) => page.load(),
                    Ok(Err(error)) => page.operation_failed(&error),
                    Err(_) => page
                        .status
                        .set_label(&gettext("A operação foi interrompida.")),
                }
            });
        });
        dialog.present(Some(&self.root));
    }

    fn create_machine(&self, request: CreateRequest) {
        if self.busy.get() || self.selected_connection() != Connection::Personal {
            return;
        }
        if let Err(error) = request.validate() {
            self.status.set_label(&error);
            return;
        }
        self.set_busy(true);
        self.cancel.set_visible(true);
        self.status.set_label(&gettext(
            "Criando disco e copiando a mídia… A máquina ficará desligada ao concluir.",
        ));
        let cancel = Arc::new(AtomicBool::new(false));
        let token = cancel.clone();
        let handler = self.cancel.connect_clicked(move |button| {
            token.store(true, Ordering::Relaxed);
            button.set_sensitive(false);
        });
        let page = self.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                Backend::open(Connection::Personal, false)?.create(&request, &cancel)
            })
            .await;
            page.cancel.disconnect(handler);
            page.cancel.set_visible(false);
            page.cancel.set_sensitive(true);
            page.set_busy(false);
            match result {
                Ok(Ok(_)) => page.load(),
                Ok(Err(error)) => page.operation_failed(&error),
                Err(_) => page
                    .status
                    .set_label(&gettext("A operação foi interrompida.")),
            }
        });
    }
}

fn state_label(state: State) -> String {
    match state {
        State::Running => gettext("Em execução"),
        State::Paused => gettext("Pausada"),
        State::Off => gettext("Desligada"),
        State::Other => gettext("Indisponível ou em transição"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a GTK display; no libvirt connections or mutations"]
    fn native_virtualization_page() {
        adw::init().unwrap();
        let page = VirtualizationPage::with_refresh(false);
        assert_eq!(page.selected_connection(), Connection::Personal);
        *page.machines.borrow_mut() = vec![
            Machine {
                uuid: "a84fdf47-f7f3-4306-9358-d2d3f38af37c".into(),
                name: "Alpha <&>".into(),
                state: State::Running,
                cpus: 2,
                memory_kib: 2048 * 1024,
                persistent: true,
            },
            Machine {
                uuid: "6a7c9167-3094-4d56-a4f1-0d75ba89ae19".into(),
                name: "Beta".into(),
                state: State::Off,
                cpus: 1,
                memory_kib: 512 * 1024,
                persistent: true,
            },
        ];
        page.render();
        let window = gtk::Window::builder()
            .default_width(720)
            .default_height(540)
            .child(&page.root)
            .build();
        window.present();
        let context = glib::MainContext::default();
        for _ in 0..20 {
            while context.pending() {
                context.iteration(false);
            }
        }
        assert!(page.list.row_at_index(1).is_some());
        let row = page
            .list
            .row_at_index(0)
            .unwrap()
            .downcast::<adw::ExpanderRow>()
            .unwrap();
        assert_eq!(row.title(), "Alpha <&>");
        assert!(!row.uses_markup());
        page.search.set_text("beta");
        page.render();
        assert!(page.list.row_at_index(1).is_none());
        let row = page
            .list
            .row_at_index(0)
            .unwrap()
            .downcast::<adw::ExpanderRow>()
            .unwrap();
        assert_eq!(row.title(), "Beta");
        page.set_busy(true);
        assert!(!page.create.is_sensitive());
        page.set_busy(false);
        assert!(page.create.is_sensitive());
        window.close();
    }
}
