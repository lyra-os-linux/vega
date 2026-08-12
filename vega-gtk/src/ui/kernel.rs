use std::{cell::RefCell, rc::Rc};

use crate::i18n::gettext;
use adw::prelude::*;

use lyra_vega_dbus::BootStatus;

#[derive(Clone)]
pub struct KernelPage {
    pub root: gtk::Widget,
    pub status: gtk::Label,
    pub installed: gtk::ListBox,
    pub available: gtk::ListBox,
    pub install: gtk::Button,
    pub remove: gtk::Button,
    pub boot_loader: gtk::Label,
    pub boot_default: gtk::Label,
    pub boot_timeout: gtk::Label,
    pub boot_cmdline: gtk::Label,
    pub boot_entries: gtk::Label,
    pub boot_entry: gtk::DropDown,
    pub boot_timeout_input: gtk::SpinButton,
    pub boot_cmdline_input: gtk::Entry,
    pub apply_boot: gtk::Button,
    installed_items: Rc<RefCell<Vec<String>>>,
    available_items: Rc<RefCell<Vec<String>>>,
}

impl KernelPage {
    pub fn new() -> Self {
        let status = gtk::Label::builder()
            .label(gettext("Carregando informações do kernel…"))
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        let installed = list();
        let available = list();
        installed.add_css_class("kernel-list");
        available.add_css_class("kernel-list");
        let install = gtk::Button::builder()
            .label(gettext("Instalar selecionado"))
            .sensitive(false)
            .css_classes(["suggested-action"])
            .build();
        let remove = gtk::Button::builder()
            .label(gettext("Remover selecionado"))
            .sensitive(false)
            .css_classes(["destructive-action"])
            .build();
        let installed_items = Rc::new(RefCell::new(Vec::<String>::new()));
        let available_items = Rc::new(RefCell::new(Vec::<String>::new()));
        let boot_loader = value(&gettext("Carregando…"));
        let boot_default = value(&gettext("Carregando…"));
        let boot_timeout = value(&gettext("Carregando…"));
        let boot_cmdline = value(&gettext("Carregando…"));
        let boot_entries = value(&gettext("Carregando…"));
        let boot_entry = gtk::DropDown::from_strings(&[]);
        let boot_timeout_input = gtk::SpinButton::with_range(0.0, 120.0, 1.0);
        let boot_cmdline_input = gtk::Entry::builder().hexpand(true).build();
        let apply_boot = gtk::Button::builder()
            .label(gettext("Aplicar bootloader"))
            .halign(gtk::Align::End)
            .css_classes(["suggested-action"])
            .build();

        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.add_css_class("content-page");
        content.add_css_class("compact-page");
        content.append(&status);
        let installed_column = gtk::Box::new(gtk::Orientation::Vertical, 8);
        installed_column.set_hexpand(true);
        installed_column.append(&section(&gettext("Kernels instalados"), &installed));
        installed_column.append(&remove);
        let available_column = gtk::Box::new(gtk::Orientation::Vertical, 8);
        available_column.set_hexpand(true);
        available_column.append(&section(&gettext("Kernels disponíveis"), &available));
        available_column.append(&install);
        let kernel_columns = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        kernel_columns.set_homogeneous(true);
        kernel_columns.append(&installed_column);
        kernel_columns.append(&available_column);
        content.append(&kernel_columns);

        let boot = adw::PreferencesGroup::builder()
            .title(gettext("Bootloader"))
            .build();
        boot.add(&property(&gettext("Detectado"), &boot_loader));
        boot.add(&property(&gettext("Entrada padrão"), &boot_default));
        boot.add(&property(&gettext("Timeout"), &boot_timeout));
        boot.add(&property(&gettext("Parâmetros"), &boot_cmdline));
        boot.add(&property(&gettext("Entradas"), &boot_entries));
        content.append(&boot);
        let boot_editor = gtk::Box::new(gtk::Orientation::Vertical, 8);
        boot_editor.add_css_class("card");
        boot_editor.append(&field(&gettext("Entrada padrão"), &boot_entry));
        boot_editor.append(&field(&gettext("Timeout em segundos"), &boot_timeout_input));
        boot_editor.append(&field(
            &gettext("Parâmetros do kernel"),
            &boot_cmdline_input,
        ));
        boot_editor.append(&apply_boot);
        content.append(&boot_editor);

        let root = gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build()
            .upcast();
        let install_button = install.clone();
        available.connect_row_selected(move |_, row| {
            install_button.set_sensitive(row.is_some());
        });
        let remove_button = remove.clone();
        let removable_items = installed_items.clone();
        installed.connect_row_selected(move |_, row| {
            remove_button.set_sensitive(kernel_removal_allowed(
                removable_items.borrow().len(),
                row.is_some(),
            ));
        });
        Self {
            root,
            status,
            installed,
            available,
            install,
            remove,
            boot_loader,
            boot_default,
            boot_timeout,
            boot_cmdline,
            boot_entries,
            boot_entry,
            boot_timeout_input,
            boot_cmdline_input,
            apply_boot,
            installed_items,
            available_items,
        }
    }

    pub fn show_installed(&self, kernels: &[String]) {
        *self.installed_items.borrow_mut() = kernels.to_vec();
        fill_list(&self.installed, kernels, &gettext("Nenhum kernel listado"));
        self.remove.set_sensitive(kernel_removal_allowed(
            kernels.len(),
            self.installed.selected_row().is_some(),
        ));
    }

    pub fn show_available(&self, kernels: &[String], installed: &[String]) {
        let kernels = available_not_installed(kernels, installed);
        fill_list(
            &self.available,
            &kernels,
            &gettext("Todos os kernels disponíveis já estão instalados"),
        );
        *self.available_items.borrow_mut() = kernels;
    }

    pub fn show_boot(&self, status: &BootStatus, entries: &[String]) {
        self.boot_loader.set_label(&status.loader);
        self.boot_default
            .set_label(&if status.default_entry.is_empty() {
                gettext("Padrão atual")
            } else {
                status.default_entry.clone()
            });
        self.boot_timeout.set_label(
            &gettext("{seconds} segundos").replace("{seconds}", &status.timeout.to_string()),
        );
        self.boot_cmdline.set_label(&if status.cmdline.is_empty() {
            gettext("Nenhum parâmetro adicional")
        } else {
            status.cmdline.clone()
        });
        let entries_text = if entries.is_empty() {
            gettext("Nenhuma entrada listada")
        } else {
            entries.join(" • ")
        };
        self.boot_entries.set_label(&entries_text);
        let mut choices = entries.to_vec();
        if !status.default_entry.is_empty() && !choices.contains(&status.default_entry) {
            choices.insert(0, status.default_entry.clone());
        }
        let values = choices.iter().map(String::as_str).collect::<Vec<_>>();
        self.boot_entry
            .set_model(Some(&gtk::StringList::new(&values)));
        if let Some(index) = choices
            .iter()
            .position(|entry| entry == &status.default_entry)
        {
            self.boot_entry.set_selected(index as u32);
        }
        self.boot_timeout_input.set_value(f64::from(status.timeout));
        self.boot_cmdline_input.set_text(&status.cmdline);
        let supported = !status.loader.is_empty() && status.loader != "não detectado";
        self.apply_boot.set_sensitive(supported);
    }

    pub fn selected_installed(&self) -> Option<String> {
        let index = self.installed.selected_row()?.index() as usize;
        self.installed_items.borrow().get(index).cloned()
    }

    pub fn selected_available(&self) -> Option<String> {
        let index = self.available.selected_row()?.index() as usize;
        self.available_items.borrow().get(index).cloned()
    }

    pub fn selected_boot_entry(&self) -> String {
        self.boot_entry
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .map(|item| item.string().to_string())
            .unwrap_or_default()
    }
}

fn list() -> gtk::ListBox {
    gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build()
}

fn field(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let field = gtk::Box::new(gtk::Orientation::Vertical, 5);
    field.append(
        &gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build(),
    );
    field.append(widget);
    field
}

fn section(title: &str, list: &gtk::ListBox) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    section.append(
        &gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["title-2"])
            .build(),
    );
    section.append(list);
    section
}

fn fill_list(list: &gtk::ListBox, values: &[String], empty: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if values.is_empty() {
        list.set_selection_mode(gtk::SelectionMode::None);
        list.append(&adw::ActionRow::builder().title(empty).build());
        return;
    }
    list.set_selection_mode(gtk::SelectionMode::Single);
    for value in values {
        list.append(
            &adw::ActionRow::builder()
                .title(gtk::glib::markup_escape_text(value))
                .build(),
        );
    }
}

fn value(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(1.0)
        .wrap(true)
        .selectable(true)
        .build()
}

fn property(title: &str, value: &gtk::Label) -> adw::ActionRow {
    value.set_hexpand(true);
    value.set_halign(gtk::Align::Fill);
    value.set_xalign(1.0);
    value.set_wrap(true);
    value.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    value.set_ellipsize(gtk::pango::EllipsizeMode::None);
    value.set_max_width_chars(-1);
    let row = adw::ActionRow::builder()
        .title(title)
        .title_lines(1)
        .build();
    row.add_suffix(value);
    row
}

fn available_not_installed(available: &[String], installed: &[String]) -> Vec<String> {
    available
        .iter()
        .filter(|candidate| {
            !installed
                .iter()
                .any(|current| current.eq_ignore_ascii_case(candidate))
        })
        .cloned()
        .collect()
}

fn kernel_removal_allowed(installed_count: usize, has_selection: bool) -> bool {
    installed_count > 1 && has_selection
}

#[cfg(test)]
mod tests {
    use super::{available_not_installed, kernel_removal_allowed};

    #[test]
    fn installed_kernels_are_removed_from_install_choices() {
        let available = vec!["kernel".into(), "kernel-lts".into(), "kernel-zen".into()];
        let installed = vec!["kernel".into(), "KERNEL-LTS".into()];
        assert_eq!(
            available_not_installed(&available, &installed),
            ["kernel-zen"]
        );
    }

    #[test]
    fn last_installed_kernel_cannot_be_removed() {
        assert!(!kernel_removal_allowed(1, true));
        assert!(!kernel_removal_allowed(2, false));
        assert!(kernel_removal_allowed(2, true));
    }
}
