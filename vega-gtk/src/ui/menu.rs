use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::i18n::gettext;
use adw::prelude::*;

use crate::dock::MenuSettings;

type ChangeHandler = Rc<dyn Fn(MenuSettings)>;

const PANEL_MENU_POSITIONS: &[&str] = &["left", "center", "right"];

fn panel_menu_position_label(id: &str) -> String {
    match id {
        "center" => gettext("Centro"),
        "right" => gettext("Direita"),
        _ => gettext("Esquerda"),
    }
}

fn id_dropdown(ids: &[&str], labels: impl Fn(&str) -> String, current: &str) -> gtk::DropDown {
    let strings: Vec<String> = ids.iter().map(|id| labels(id)).collect();
    let model = gtk::StringList::new(&strings.iter().map(String::as_str).collect::<Vec<_>>());
    let dropdown = gtk::DropDown::builder()
        .model(&model)
        .valign(gtk::Align::Center)
        .build();
    let index = ids.iter().position(|&id| id == current).unwrap_or(0);
    dropdown.set_selected(index as u32);
    dropdown
}

fn dropdown_selected<'a>(dropdown: &gtk::DropDown, ids: &'a [&str]) -> &'a str {
    ids.get(dropdown.selected() as usize)
        .copied()
        .unwrap_or(ids[0])
}

#[derive(Clone)]
pub struct MenuPage {
    pub root: gtk::Widget,
    pub status: gtk::Label,
    pub show_applications_menu: gtk::Switch,
    pub show_places_menu: gtk::Switch,
    pub show_system_menu: gtk::Switch,
    pub show_search_menu: gtk::Switch,
    pub hide_workspace_button: gtk::Switch,
    pub panel_menu_position: gtk::DropDown,
    pub show_application_icons: gtk::Switch,
    pub sort_applications_menu: gtk::Switch,
    pub open_application_submenus_sideways: gtk::Switch,
    pub show_place_bookmarks: gtk::Switch,
    pub show_place_volumes: gtk::Switch,
    suppress: Rc<Cell<bool>>,
    change_handlers: Rc<RefCell<Vec<ChangeHandler>>>,
}

impl MenuPage {
    pub fn new() -> Self {
        let status = gtk::Label::builder()
            .label(gettext("Carregando configuração…"))
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();

        let show_applications_menu = switch();
        let show_places_menu = switch();
        let show_system_menu = switch();
        let show_search_menu = switch();
        let hide_workspace_button = switch();
        let panel_menu_position =
            id_dropdown(PANEL_MENU_POSITIONS, panel_menu_position_label, "left");
        let show_application_icons = switch();
        let sort_applications_menu = switch();
        let open_application_submenus_sideways = switch();
        let show_place_bookmarks = switch();
        let show_place_volumes = switch();

        let panel_group = adw::PreferencesGroup::builder()
            .title(gettext("Menus da barra superior"))
            .build();
        panel_group.add(&property_row(
            &gettext("Menu Aplicativos"),
            &show_applications_menu,
        ));
        panel_group.add(&property_row(&gettext("Menu Locais"), &show_places_menu));
        panel_group.add(&property_row(&gettext("Menu Sistema"), &show_system_menu));
        panel_group.add(&property_row(&gettext("Menu Busca"), &show_search_menu));
        panel_group.add(&property_row(
            &gettext("Ocultar botão de áreas de trabalho"),
            &hide_workspace_button,
        ));
        panel_group.add(&property_row(
            &gettext("Posição na barra"),
            &panel_menu_position,
        ));

        let panel_content_group = adw::PreferencesGroup::builder()
            .title(gettext("Conteúdo dos menus"))
            .build();
        panel_content_group.add(&property_row(
            &gettext("Ícones dos aplicativos"),
            &show_application_icons,
        ));
        panel_content_group.add(&property_row(
            &gettext("Ordem alfabética"),
            &sort_applications_menu,
        ));
        panel_content_group.add(&property_row(
            &gettext("Submenus laterais"),
            &open_application_submenus_sideways,
        ));
        panel_content_group.add(&property_row(
            &gettext("Marcadores em Locais"),
            &show_place_bookmarks,
        ));
        panel_content_group.add(&property_row(
            &gettext("Dispositivos em Locais"),
            &show_place_volumes,
        ));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.append(&status);
        content.append(&panel_group);
        content.append(&panel_content_group);

        let root = gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build()
            .upcast();

        let page = Self {
            root,
            status,
            show_applications_menu,
            show_places_menu,
            show_system_menu,
            show_search_menu,
            hide_workspace_button,
            panel_menu_position,
            show_application_icons,
            sort_applications_menu,
            open_application_submenus_sideways,
            show_place_bookmarks,
            show_place_volumes,
            suppress: Rc::new(Cell::new(false)),
            change_handlers: Rc::new(RefCell::new(Vec::new())),
        };
        page.wire_changed_signals();
        page
    }

    /// Chamado a cada mudança de controle (exceto durante `show`, quando os
    /// valores são carregados programaticamente) para aplicar de imediato,
    /// no mesmo padrão da aba Dock.
    pub fn connect_changed(&self, handler: impl Fn(MenuSettings) + 'static) {
        self.change_handlers.borrow_mut().push(Rc::new(handler));
    }

    fn emit_changed(&self) {
        if self.suppress.get() {
            return;
        }
        let settings = self.selected();
        for handler in self.change_handlers.borrow().iter() {
            handler(settings.clone());
        }
    }

    fn wire_changed_signals(&self) {
        let page = self.clone();
        self.show_applications_menu
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_places_menu
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_system_menu
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_search_menu
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.hide_workspace_button
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.panel_menu_position
            .connect_selected_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_application_icons
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.sort_applications_menu
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.open_application_submenus_sideways
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_place_bookmarks
            .connect_active_notify(move |_| page.emit_changed());
        let page = self.clone();
        self.show_place_volumes
            .connect_active_notify(move |_| page.emit_changed());
    }

    pub fn show(&self, settings: &MenuSettings) {
        self.suppress.set(true);
        self.show_applications_menu
            .set_active(settings.show_applications_menu);
        self.show_places_menu.set_active(settings.show_places_menu);
        self.show_system_menu.set_active(settings.show_system_menu);
        self.show_search_menu.set_active(settings.show_search_menu);
        self.hide_workspace_button
            .set_active(settings.hide_workspace_button);
        self.panel_menu_position.set_selected(
            PANEL_MENU_POSITIONS
                .iter()
                .position(|&id| id == settings.panel_menu_position)
                .unwrap_or(0) as u32,
        );
        self.show_application_icons
            .set_active(settings.show_application_icons);
        self.sort_applications_menu
            .set_active(settings.sort_applications_menu);
        self.open_application_submenus_sideways
            .set_active(settings.open_application_submenus_sideways);
        self.show_place_bookmarks
            .set_active(settings.show_place_bookmarks);
        self.show_place_volumes
            .set_active(settings.show_place_volumes);
        self.suppress.set(false);
        self.status
            .set_label(&gettext("Configuração atual carregada"));
    }

    pub fn selected(&self) -> MenuSettings {
        MenuSettings {
            show_applications_menu: self.show_applications_menu.is_active(),
            show_places_menu: self.show_places_menu.is_active(),
            show_system_menu: self.show_system_menu.is_active(),
            show_search_menu: self.show_search_menu.is_active(),
            hide_workspace_button: self.hide_workspace_button.is_active(),
            panel_menu_position: dropdown_selected(&self.panel_menu_position, PANEL_MENU_POSITIONS)
                .to_string(),
            show_application_icons: self.show_application_icons.is_active(),
            sort_applications_menu: self.sort_applications_menu.is_active(),
            open_application_submenus_sideways: self.open_application_submenus_sideways.is_active(),
            show_place_bookmarks: self.show_place_bookmarks.is_active(),
            show_place_volumes: self.show_place_volumes.is_active(),
        }
    }
}

impl Default for MenuPage {
    fn default() -> Self {
        Self::new()
    }
}

fn switch() -> gtk::Switch {
    gtk::Switch::builder()
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build()
}

fn property_row(title: &str, widget: &impl IsA<gtk::Widget>) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).build();
    row.add_suffix(widget);
    row
}
