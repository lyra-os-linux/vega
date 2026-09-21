use crate::i18n::gettext;
use adw::prelude::*;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use vega_virtualization::{
    Action, Backend, Connection, CreateRequest, EditDetails, Firmware, Machine, MediaKind, State,
};

#[derive(Clone)]
pub struct VirtualizationPage {
    pub root: gtk::Widget,
    connection: gtk::DropDown,
    search: gtk::SearchEntry,
    refresh: gtk::Button,
    create: gtk::Button,
    cancel: gtk::Button,
    status: gtk::Label,
    progress: gtk::ProgressBar,
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
        let progress = gtk::ProgressBar::new();
        progress.set_visible(false);
        progress.set_show_text(true);
        body.append(&progress);
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
            progress,
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
        self.status
            .set_label(&crate::virtualization_messages::translate(error));
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
            let shortcut = gtk::Button::with_label(&gettext("Atalho da tela"));
            shortcut.set_tooltip_text(Some(&gettext("Adiciona a tela ao menu de aplicativos. Se a máquina estiver desligada, o console aguardará o início pelo Vega.")));
            let page = self.clone();
            let machine_for_shortcut = machine.clone();
            shortcut.connect_clicked(move |_| page.create_shortcut(&machine_for_shortcut));
            actions.insert(&shortcut, -1);
            if machine.state == State::Off && machine.persistent {
                let configure = gtk::Button::with_label(&gettext("Editar máquina"));
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

    fn confirm_removal(&self, machine: &Machine) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        let connection = self.selected_connection();
        let page = self.clone();
        let machine = machine.clone();
        glib::spawn_future_local(async move {
            let uuid = machine.uuid.clone();
            let result =
                gio::spawn_blocking(move || Backend::open(connection, true)?.removal_plan(&uuid))
                    .await;
            page.set_busy(false);
            if page.selected_connection() != connection {
                return;
            }
            let dialog=adw::AlertDialog::builder().heading(&machine.name)
                .body(gettext("Remover esta máquina? Por padrão, os discos e arquivos locais serão preservados.")).build();
            let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
            let erase = gtk::CheckButton::with_label(&gettext(
                "Apagar também os discos e arquivos locais listados abaixo",
            ));
            if let Some(label) = erase.child().and_downcast::<gtk::Label>() {
                label.set_wrap(true);
                label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                label.set_max_width_chars(48);
            }
            erase.set_active(false);
            let plan = match result {
                Ok(Ok(plan)) => Some(plan),
                Ok(Err(error)) => {
                    form.append(
                        &gtk::Label::builder()
                            .label(crate::virtualization_messages::translate(&error))
                            .wrap(true)
                            .build(),
                    );
                    None
                }
                Err(_) => None,
            };
            erase.set_sensitive(plan.as_ref().is_some_and(|p| !p.files.is_empty()));
            form.append(&erase);
            if let Some(plan) = &plan {
                let files = gtk::Label::builder()
                    .label(plan.files.join("\n"))
                    .selectable(true)
                    .wrap(true)
                    .wrap_mode(gtk::pango::WrapMode::WordChar)
                    .max_width_chars(48)
                    .xalign(0.0)
                    .build();
                form.append(&files);
            }
            form.append(&gtk::Label::builder().label(gettext("A exclusão é permanente. Discos compartilhados e mídias externas não serão apagados.")).wrap(true).build());
            let scroll = gtk::ScrolledWindow::builder()
                .child(&form)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_width(true)
                .min_content_height(200)
                .max_content_height(320)
                .propagate_natural_height(true)
                .build();
            dialog.set_extra_child(Some(&scroll));
            dialog.add_responses(&[
                ("cancel", &gettext("Cancelar")),
                ("remove", &gettext("Remover")),
            ]);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
            let page2 = page.clone();
            dialog.connect_response(None, move |_, response| {
                if response != "remove" || page2.selected_connection() != connection {
                    return;
                }
                if erase.is_active() {
                    if let Some(plan) = plan.clone() {
                        let uuid = machine.uuid.clone();
                        page2.run_edit(connection, move |backend| {
                            backend.remove_with_files(&uuid, &plan)
                        });
                    }
                } else {
                    page2.execute(&machine, Action::Remove);
                }
            });
            dialog.present(Some(&page.root));
        });
    }

    fn confirm(&self, machine: &Machine, action: Action) {
        if action == Action::Remove {
            self.confirm_removal(machine);
            return;
        }
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
            Action::Remove => gettext(
                "Remover a definição desta máquina? Os discos e dados do firmware serão preservados.",
            ),
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

    fn create_shortcut(&self, machine: &Machine) {
        if self.busy.get() {
            return;
        }
        let connection = self.selected_connection();
        let machine = machine.clone();
        let page = self.clone();
        self.set_busy(true);
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
                let xdg = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from);
                vega_virtualization::create_shortcut(
                    connection,
                    &machine.uuid,
                    &machine.name,
                    xdg.as_deref(),
                    &std::path::PathBuf::from(home),
                )
            })
            .await;
            page.set_busy(false);
            match result {
                Ok(Ok(_)) => page
                    .status
                    .set_label(&gettext("Atalho criado no menu de aplicativos.")),
                Ok(Err(error)) => page.operation_failed(&error),
                Err(_) => page.operation_failed(&gettext("A operação foi interrompida.")),
            }
        });
    }

    fn create_dialog(&self) {
        if self.busy.get() || self.selected_connection() != Connection::Personal {
            return;
        }
        let dialog = adw::AlertDialog::builder().heading(gettext("Nova máquina pessoal"))
            .body(gettext("Instale por ISO ou importe uma cópia de um disco. Para importar, desligue a máquina de origem. Os arquivos originais serão preservados."))
            .build();
        let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let name = gtk::Entry::builder()
            .placeholder_text(gettext("Nome da máquina"))
            .build();
        name.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Nome da máquina",
        ))]);
        form.append(&name);
        let kind = gtk::DropDown::from_strings(&[
            &gettext("Instalar por ISO"),
            &gettext("Importar QCOW2"),
            &gettext("Importar RAW"),
        ]);
        kind.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Origem da máquina",
        ))]);
        let row = adw::ActionRow::builder()
            .title(gettext("Origem da máquina"))
            .build();
        kind.set_valign(gtk::Align::Center);
        row.add_suffix(&kind);
        form.append(&row);
        let firmware = gtk::DropDown::from_strings(&["BIOS", "UEFI"]);
        firmware.update_property(&[gtk::accessible::Property::Label(&gettext("Firmware"))]);
        let row = adw::ActionRow::builder().title(gettext("Firmware")).build();
        firmware.set_valign(gtk::Align::Center);
        row.add_suffix(&firmware);
        form.append(&row);
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
        let media = gtk::Button::with_label(&gettext("Selecionar arquivo…"));
        let path = Rc::new(RefCell::new(None));
        let selected = path.clone();
        let import_disk = disk.clone();
        let selected_path = path.clone();
        let media_button = media.clone();
        kind.connect_selected_notify(move |kind| {
            import_disk.set_sensitive(kind.selected() == 0);
            *selected_path.borrow_mut() = None;
            media_button.set_label(&gettext("Selecionar arquivo…"));
        });
        let root = self.root.clone();
        media.connect_clicked(move |button| {
            let chooser = gtk::FileDialog::builder()
                .title(gettext("Selecionar arquivo"))
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
            let Some(source) = path.borrow().clone() else {
                page.status
                    .set_label(&gettext("Selecione a mídia antes de criar a máquina."));
                return;
            };
            let request = CreateRequest {
                name: name.text().to_string(),
                source,
                kind: match kind.selected() {
                    1 => MediaKind::Qcow2,
                    2 => MediaKind::Raw,
                    _ => MediaKind::Iso,
                },
                firmware: if firmware.selected() == 1 {
                    Firmware::Uefi
                } else {
                    Firmware::Bios
                },
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
        self.set_busy(true);
        let connection = self.selected_connection();
        let machine = machine.clone();
        let page = self.clone();
        glib::spawn_future_local(async move {
            let uuid = machine.uuid.clone();
            let result =
                gio::spawn_blocking(move || Backend::open(connection, false)?.edit_details(&uuid))
                    .await;
            page.set_busy(false);
            if page.selected_connection() != connection {
                return;
            }
            match result {
                Ok(Ok(details)) => page.edit_dialog(&machine, details),
                Ok(Err(error)) => page.operation_failed(&error),
                Err(_) => page
                    .status
                    .set_label(&gettext("A operação foi interrompida.")),
            }
        });
    }

    fn run_edit(
        &self,
        connection: Connection,
        operation: impl FnOnce(Backend) -> Result<(), String> + Send + 'static,
    ) {
        if self.busy.get() || self.selected_connection() != connection {
            return;
        }
        self.set_busy(true);
        self.status.set_label(&gettext("Executando operação…"));
        let page = self.clone();
        glib::spawn_future_local(async move {
            let result =
                gio::spawn_blocking(move || operation(Backend::open(connection, false)?)).await;
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

    fn edit_dialog(&self, machine: &Machine, details: EditDetails) {
        if self.busy.get() {
            return;
        }
        let connection = self.selected_connection();
        let dialog = adw::Dialog::builder()
            .title(&machine.name)
            .content_width(640)
            .content_height(580)
            .build();
        let form = gtk::Box::new(gtk::Orientation::Vertical, 8);
        form.append(&gtk::Label::builder()
            .label(gettext("A máquina deve estar desligada. Cada botão aplica apenas a alteração indicada."))
            .wrap(true).xalign(0.0).build());
        let name = adw::EntryRow::builder()
            .title(gettext("Nome da máquina"))
            .text(&machine.name)
            .build();
        let rename = gtk::Button::with_label(&gettext("Renomear"));
        rename.set_valign(gtk::Align::Center);
        name.add_suffix(&rename);
        form.append(&name);
        let page = self.clone();
        let uuid = machine.uuid.clone();
        let close = dialog.clone();
        rename.connect_clicked(move |_| {
            let value = name.text().to_string();
            let uuid = uuid.clone();
            close.close();
            page.run_edit(connection, move |backend| backend.rename(&uuid, &value));
        });

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

        for disk in details.disks {
            let row = adw::ActionRow::builder().use_markup(false).build();
            row.set_title(&format!("{} · {}", gettext("Disco"), disk.target));
            if let Some(capacity) = disk.capacity {
                row.set_subtitle(&format!(
                    "{:.2} GiB",
                    capacity as f64 / (1_u64 << 30) as f64
                ));
                let minimum = (capacity / (1 << 30) + 1) as f64;
                let size = gtk::SpinButton::with_range(minimum.min(2048.0), 2048.0, 1.0);
                size.set_valign(gtk::Align::Center);
                size.update_property(&[gtk::accessible::Property::Label(&gettext(
                    "Nova capacidade (GiB)",
                ))]);
                let grow = gtk::Button::with_label(&gettext("Ampliar"));
                grow.set_valign(gtk::Align::Center);
                grow.set_sensitive(minimum <= 2048.0);
                let page = self.clone();
                let uuid = machine.uuid.clone();
                let target = disk.target.clone();
                let close = dialog.clone();
                let selected_size = size.clone();
                grow.connect_clicked(move |_| {
                    let bytes = (selected_size.value_as_int() as u64) << 30;
                    let confirm = adw::AlertDialog::builder()
                        .heading(gettext("Ampliar disco?"))
                        .body(format!("{:.2} GiB → {} GiB\n{}", capacity as f64 / (1_u64 << 30) as f64, bytes >> 30,
                            gettext("O disco não poderá ser reduzido pelo Vega. Depois, expanda a partição dentro do sistema convidado para usar o novo espaço.")))
                        .build();
                    confirm.add_responses(&[("cancel", &gettext("Cancelar")), ("grow", &gettext("Ampliar"))]);
                    confirm.set_default_response(Some("cancel"));
                    confirm.set_close_response("cancel");
                    let page2 = page.clone(); let uuid = uuid.clone(); let target = target.clone(); let close = close.clone();
                    confirm.connect_response(None, move |_, response| {
                        if response == "grow" {
                            close.close(); let uuid=uuid.clone(); let target=target.clone();
                            page2.run_edit(connection, move |backend| backend.grow_disk(&uuid,&target,bytes));
                        }
                    });
                    confirm.present(Some(&page.root));
                });
                row.add_suffix(&size);
                row.add_suffix(&grow);
            } else {
                row.set_subtitle(&crate::virtualization_messages::translate(
                    disk.unavailable.as_deref().unwrap_or(""),
                ));
            }
            form.append(&row);
        }
        for target in details.media {
            let row = adw::ActionRow::builder()
                .title(format!("{} · {target}", gettext("Leitor de ISO")))
                .subtitle(gettext("Ejetar a mídia preserva o arquivo ISO no disco."))
                .build();
            let eject = gtk::Button::with_label(&gettext("Ejetar ISO"));
            eject.set_valign(gtk::Align::Center);
            let page = self.clone();
            let uuid = machine.uuid.clone();
            let close = dialog.clone();
            eject.connect_clicked(move |_| {
                close.close();
                let uuid = uuid.clone();
                let target = target.clone();
                page.run_edit(connection, move |backend| backend.eject_iso(&uuid, &target));
            });
            row.add_suffix(&eject);
            form.append(&row);
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&form)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content.set_margin_start(16);
        content.set_margin_end(16);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.append(&scroll);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        footer.set_halign(gtk::Align::End);
        let close = gtk::Button::with_label(&gettext("Fechar"));
        let dismiss = dialog.clone();
        close.connect_clicked(move |_| {
            dismiss.close();
        });
        let save = gtk::Button::with_label(&gettext("Aplicar CPU e memória"));
        save.add_css_class("suggested-action");
        footer.append(&close);
        footer.append(&save);
        content.append(&footer);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&content));
        dialog.set_child(Some(&toolbar));
        let page = self.clone();
        let uuid = machine.uuid.clone();
        let dismiss = dialog.clone();
        save.connect_clicked(move |_| {
            if page.busy.get() || page.selected_connection() != connection {
                return;
            }
            dismiss.close();
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
            self.operation_failed(&error);
            return;
        }
        self.set_busy(true);
        self.cancel.set_visible(true);
        self.progress.set_fraction(0.0);
        self.progress.set_visible(true);
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
        let fraction = Arc::new(AtomicU64::new(0));
        let meter = fraction.clone();
        let bar = self.progress.clone();
        let timer = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            bar.set_fraction(meter.load(Ordering::Relaxed) as f64 / 10000.0);
            glib::ControlFlow::Continue
        });
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                Backend::open(Connection::Personal, false)?.create_with_progress(
                    &request,
                    &cancel,
                    |done, total| {
                        fraction
                            .store(done.saturating_mul(10000) / total.max(1), Ordering::Relaxed);
                    },
                )
            })
            .await;
            timer.remove();
            page.progress.set_visible(false);
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
        // Opening the editor must be side-effect free; drive capacity starts
        // strictly above the existing size and ISO removal is explicit.
        page.edit_dialog(
            &page.machines.borrow()[1],
            EditDetails {
                disks: vec![vega_virtualization::Disk {
                    target: "vda".into(),
                    path: "/fixture/disk.qcow2".into(),
                    capacity: Some(4_u64 << 30),
                    unavailable: None,
                }],
                media: vec!["sda".into()],
            },
        );
        for _ in 0..40 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fn widgets(w: &gtk::Widget, out: &mut Vec<gtk::Widget>) {
            out.push(w.clone());
            let mut child = w.first_child();
            while let Some(c) = child {
                widgets(&c, out);
                child = c.next_sibling();
            }
        }
        let mut all = vec![];
        for top in gtk::Window::list_toplevels() {
            widgets(&top, &mut all);
        }
        // The old alert's narrow extra-child viewport clipped the disk controls
        // even on a desktop display. A form dialog must allocate usable width.
        let editor = all
            .iter()
            .filter_map(|w| w.downcast_ref::<adw::Dialog>())
            .find(|d| d.title().as_str() == "Beta")
            .expect("VM editor");
        assert!(editor.width() >= 500, "editor width: {}", editor.width());
        let grow = all
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Button>())
            .find(|b| b.label().as_deref() == Some(gettext("Ampliar").as_str()))
            .unwrap();
        let bounds = grow.compute_bounds(editor).expect("grow button in editor");
        assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= editor.width() as f32);
        let buttons: Vec<_> = all
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Button>())
            .filter_map(|b| b.label())
            .collect();
        for label in ["Renomear", "Ampliar", "Ejetar ISO", "Aplicar CPU e memória"] {
            assert!(
                buttons.iter().any(|s| s.as_str() == gettext(label)),
                "{label}"
            );
        }
        assert!(
            all.iter()
                .filter_map(|w| w.downcast_ref::<gtk::SpinButton>())
                .any(|s| s.adjustment().lower() == 5.0 && s.value() == 5.0)
        );
        window.close();
    }
}
