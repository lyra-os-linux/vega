use crate::i18n::gettext;
use adw::prelude::*;

use super::{DockPage, MenuPage, ScreensaverPage};
use crate::appearance::Theme;
use crate::dock::DesktopProfile;
use std::{cell::Cell, rc::Rc};

/// Visão geral em cartões, com atalhos para os aplicativos do GNOME e
/// páginas internas para as preferências de aparência e do ambiente Lyra.
#[derive(Clone)]
pub struct ScreenPage {
    pub root: gtk::Widget,
    pub screensaver: ScreensaverPage,
    pub menu: MenuPage,
    pub dock: DockPage,
}

impl ScreenPage {
    pub fn new() -> Self {
        let screensaver = ScreensaverPage::new();
        let menu = MenuPage::new();
        let dock = DockPage::new();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .vexpand(true)
            .build();
        stack.add_css_class("content-page");
        let overview = super::personalization::PersonalizationOverview::new(&stack);
        let (appearance, profile) = appearance_pages(
            &overview.menu,
            &overview.dock,
            crate::dock::is_installed(),
            &dock,
            &menu,
        );
        stack.add_named(&overview.root, Some("overview"));
        for (name, title, page) in [
            ("appearance", gettext("Tema"), &appearance),
            ("profile", gettext("Perfil da área de trabalho"), &profile),
            (
                "screensaver",
                gettext("Tela de bloqueio"),
                &screensaver.root,
            ),
            ("menu", gettext("Menu"), &menu.root),
            ("dock", gettext("Dock"), &dock.root),
        ] {
            stack.add_named(&detail_page(&stack, &title, page), Some(name));
        }
        stack.set_visible_child_name("overview");

        Self {
            root: stack.upcast(),
            screensaver,
            menu,
            dock,
        }
    }
}

impl Default for ScreenPage {
    fn default() -> Self {
        Self::new()
    }
}

fn detail_page(stack: &gtk::Stack, title: &str, page: &gtk::Widget) -> gtk::Widget {
    let back = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(gettext("Voltar à personalização"))
        .valign(gtk::Align::Center)
        .build();
    back.update_property(&[gtk::accessible::Property::Label(&gettext(
        "Voltar à personalização",
    ))]);
    let stack = stack.downgrade();
    back.connect_clicked(move |_| {
        if let Some(stack) = stack.upgrade() {
            stack.set_visible_child_name("overview");
        }
    });
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.append(&back);
    header.append(
        &gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["title-1"])
            .build(),
    );
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.append(&header);
    page.set_vexpand(true);
    content.append(page);
    content.upcast()
}

/// O tema escreve direto em `org.gnome.desktop.interface` (veja
/// `crate::appearance`): não é preferência do Vega, é a mesma
/// configuração do painel Aparência do GNOME — muda o Shell, o Nautilus e
/// qualquer app libadwaita em execução, não só a janela do Vega.
fn appearance_pages(
    menu_tab: &gtk::Button,
    dock_tab: &gtk::Button,
    sheliak_available: bool,
    dock_page: &DockPage,
    menu_page: &MenuPage,
) -> (gtk::Widget, gtk::Widget) {
    let unavailable = !crate::appearance::schema_available();

    let theme_group = adw::PreferencesGroup::builder()
        .title(gettext("Tema"))
        .valign(gtk::Align::Start)
        .build();
    theme_group.set_margin_top(12);

    let cards = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    cards.set_homogeneous(true);
    cards.set_hexpand(true);
    cards.set_valign(gtk::Align::Start);

    // O GSetting pode estar em "default" (segue o sistema) mesmo sem esta UI
    // oferecer mais essa opção — nesse caso o card selecionado reflete a
    // aparência efetiva atual (resolvida pelo libadwaita), não força um valor.
    let resolved_dark = match crate::appearance::current_theme() {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => adw::StyleManager::default().is_dark(),
    };

    let light_card = theme_card(false, gettext("Claro"), None);
    let dark_card = theme_card(true, gettext("Escuro"), Some(&light_card));
    light_card.set_sensitive(!unavailable);
    dark_card.set_sensitive(!unavailable);
    light_card.set_active(!resolved_dark);
    dark_card.set_active(resolved_dark);

    light_card.connect_toggled(|button| {
        if button.is_active() {
            crate::appearance::apply_theme(Theme::Light);
        }
    });
    dark_card.connect_toggled(|button| {
        if button.is_active() {
            crate::appearance::apply_theme(Theme::Dark);
        }
    });

    cards.append(&light_card);
    cards.append(&dark_card);
    theme_group.add(&cards);

    let lyra_profile = profile_card(
        &gettext("Lyra"),
        &gettext("GNOME mais Dock e Menu do Lyra."),
        DesktopProfile::Lyra,
        None,
    );
    let ubuntu_profile = profile_card(
        &gettext("Ubuntu"),
        &gettext(
            "Dock lateral estendido, aplicativos no topo e barra superior sem menus nem busca.",
        ),
        DesktopProfile::Ubuntu,
        Some(&lyra_profile),
    );
    let vanilla_profile = profile_card(
        &gettext("Gnome Vanila"),
        &gettext("Usa a experiência padrão do GNOME."),
        DesktopProfile::GnomeVanilla,
        Some(&lyra_profile),
    );
    let windows10_profile = profile_card(
        &gettext("Lyra Clássico"),
        &gettext("Painel inferior à esquerda e menu L com lista de aplicativos e blocos fixados."),
        DesktopProfile::Windows10,
        Some(&lyra_profile),
    );
    let windows11_profile = profile_card(
        &gettext("Lyra Central"),
        &gettext(
            "Painel inferior centralizado e menu L com pesquisa e grade de aplicativos fixados.",
        ),
        DesktopProfile::Windows11,
        Some(&lyra_profile),
    );
    let macos_profile = profile_card(
        &gettext("Lyra Flutuante"),
        &gettext(
            "Dock inferior flutuante e centralizado, com ampliação dos ícones e menu L na barra superior.",
        ),
        DesktopProfile::Macos,
        Some(&lyra_profile),
    );
    macos_profile.set_sensitive(sheliak_available && crate::dock::supports_macos_profile());
    lyra_profile.set_sensitive(sheliak_available);
    ubuntu_profile.set_sensitive(sheliak_available);
    windows10_profile.set_sensitive(sheliak_available);
    windows11_profile.set_sensitive(sheliak_available);
    let choices = [
        (DesktopProfile::Lyra, lyra_profile.clone()),
        (DesktopProfile::Ubuntu, ubuntu_profile.clone()),
        (DesktopProfile::GnomeVanilla, vanilla_profile.clone()),
        (DesktopProfile::Windows10, windows10_profile.clone()),
        (DesktopProfile::Windows11, windows11_profile.clone()),
        (DesktopProfile::Macos, macos_profile.clone()),
    ];
    let active = crate::dock::current_profile();
    for (profile, button) in &choices {
        button.set_active(*profile == active);
    }
    let status = gtk::Label::builder().wrap(true).xalign(0.0).build();
    status.add_css_class("error");
    let suppress = Rc::new(Cell::new(false));
    for (profile, button) in &choices {
        let profile = *profile;
        let menu_tab = menu_tab.clone();
        let dock_tab = dock_tab.clone();
        let dock_page = dock_page.clone();
        let menu_page = menu_page.clone();
        let status = status.clone();
        let suppress = suppress.clone();
        let peers = choices
            .iter()
            .map(|(profile, button)| (*profile, button.downgrade()))
            .collect::<Vec<_>>();
        button.connect_toggled(move |button| {
            if !button.is_active() || suppress.get() {
                return;
            }
            match crate::dock::apply_profile(profile) {
                Ok(()) => {
                    status.set_label("");
                    let enabled = profile != DesktopProfile::GnomeVanilla;
                    menu_tab.set_sensitive(enabled);
                    dock_tab.set_sensitive(enabled);
                    if let Some(settings) = crate::dock::current() {
                        dock_page.show(&settings);
                    }
                    if let Some(settings) = crate::dock::current_menu() {
                        menu_page.show(&settings);
                    }
                }
                Err(error) => {
                    status.set_label(&error.to_string());
                    suppress.set(true);
                    let active = crate::dock::current_profile();
                    for (profile, peer) in &peers {
                        if let Some(peer) = peer.upgrade() {
                            peer.set_active(*profile == active);
                        }
                    }
                    suppress.set(false);
                }
            }
        });
    }

    let profiles = gtk::FlowBox::builder()
        .homogeneous(true)
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .row_spacing(16)
        .column_spacing(16)
        .valign(gtk::Align::Start)
        .build();
    for (_, button) in &choices {
        button.set_size_request(240, -1);
        profiles.insert(button, -1);
    }

    let profile_group = adw::PreferencesGroup::builder()
        .title(gettext("Perfil da área de trabalho"))
        .valign(gtk::Align::Start)
        .build();
    profile_group.add(&profiles);
    profile_group.add(&status);
    // Refresh the selected card after editing the dock or returning to this page.
    let peers = choices
        .iter()
        .map(|(profile, button)| (*profile, button.downgrade()))
        .collect::<Vec<_>>();
    profile_group.connect_map(move |_| {
        suppress.set(true);
        let active = crate::dock::current_profile();
        for (profile, peer) in &peers {
            if let Some(peer) = peer.upgrade() {
                peer.set_active(*profile == active);
            }
        }
        suppress.set(false);
    });

    let theme_content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    theme_content.set_valign(gtk::Align::Start);
    if unavailable {
        theme_content.append(
            &gtk::Label::builder()
                .label(gettext(
                    "Este sistema não tem os esquemas do GNOME para aparência; as opções abaixo ficam desativadas.",
                ))
                .xalign(0.0)
                .wrap(true)
                .css_classes(["dim-label"])
                .build(),
        );
    }
    theme_content.append(&theme_group);

    let profile_content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    profile_content.set_valign(gtk::Align::Start);
    profile_content.append(&profile_group);
    let desktop_group = adw::PreferencesGroup::builder()
        .title(gettext("Área de trabalho"))
        .build();
    let desktop_state = crate::dock::desktop_icons_state();
    let desktop_switch = adw::SwitchRow::builder()
        .title(gettext("Área de trabalho ativa"))
        .subtitle(if desktop_state.is_some() {
            gettext("Mostrar arquivos, pastas, discos e lixeira. Desativar preserva seus arquivos.")
        } else {
            gettext("Ícones na área de trabalho indisponíveis.")
        })
        .active(desktop_state.unwrap_or(false))
        .sensitive(desktop_state.is_some())
        .build();
    let desktop_status = gtk::Label::builder().wrap(true).xalign(0.0).build();
    desktop_status.add_css_class("error");
    let updating = Rc::new(Cell::new(false));
    let status = desktop_status.clone();
    desktop_switch.connect_active_notify(move |row| {
        if updating.replace(true) {
            return;
        }
        match crate::dock::set_desktop_icons_enabled(row.is_active()) {
            Ok(()) => status.set_label(""),
            Err(error) => status.set_label(&error.to_string()),
        }
        let actual = crate::dock::desktop_icons_state();
        row.set_active(actual.unwrap_or(false));
        row.set_sensitive(actual.is_some());
        updating.set(false);
    });
    desktop_group.add(&desktop_switch);
    desktop_group.add(&desktop_status);
    profile_content.append(&desktop_group);

    let theme_page = gtk::ScrolledWindow::builder()
        .child(&theme_content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
        .upcast();
    let profile_page = gtk::ScrolledWindow::builder()
        .child(&profile_content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
        .upcast();
    (theme_page, profile_page)
}

fn profile_card(
    title: &str,
    description: &str,
    profile: DesktopProfile,
    group: Option<&gtk::ToggleButton>,
) -> gtk::ToggleButton {
    let title = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .css_classes(["heading"])
        .build();
    let description = gtk::Label::builder()
        .label(description)
        .xalign(0.0)
        .wrap(true)
        .max_width_chars(30)
        .css_classes(["dim-label"])
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.append(&profile_preview(profile));
    content.append(&title);
    content.append(&description);
    let button = gtk::ToggleButton::builder()
        .child(&content)
        .css_classes(["flat", "vega-profile-card"])
        .build();
    button.update_property(&[
        gtk::accessible::Property::Label(&title.text()),
        gtk::accessible::Property::Description(&description.text()),
    ]);
    if let Some(group) = group {
        button.set_group(Some(group));
    }
    button
}

/// Ilustração compacta do desktop de cada perfil. É construída com widgets e
/// CSS (sem imagem externa): Lyra tem painel flutuante e dock lateral; GNOME
/// Vanilla tem painel colado ao topo e dash central inferior.
fn profile_preview(profile: DesktopProfile) -> gtk::Widget {
    if matches!(
        profile,
        DesktopProfile::Windows10 | DesktopProfile::Windows11
    ) {
        return windows_profile_preview(profile);
    }
    let lyra = profile == DesktopProfile::Lyra;
    let ubuntu = profile == DesktopProfile::Ubuntu;
    let macos = profile == DesktopProfile::Macos;
    let desktop = gtk::Box::new(gtk::Orientation::Vertical, 0);
    desktop.add_css_class("vega-profile-preview");

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&desktop));

    let panel = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    panel.add_css_class("vega-profile-preview-panel");
    panel.set_halign(gtk::Align::Fill);
    panel.set_valign(gtk::Align::Start);
    for _ in 0..3 {
        let item = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        item.add_css_class("vega-profile-preview-item");
        panel.append(&item);
    }
    if lyra {
        panel.add_css_class("vega-profile-preview-panel-lyra");
        panel.set_margin_top(7);
        panel.set_margin_start(9);
        panel.set_margin_end(9);
    } else {
        panel.add_css_class("vega-profile-preview-panel-gnome");
    }
    if macos {
        panel.prepend(&gtk::Label::new(Some("L")));
    }
    overlay.add_overlay(&panel);

    let dock = gtk::Box::new(
        if lyra || ubuntu {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        },
        4,
    );
    dock.add_css_class("vega-profile-preview-dock");
    dock.add_css_class(if lyra {
        "vega-profile-preview-dock-lyra"
    } else if ubuntu {
        "vega-profile-preview-dock-ubuntu"
    } else if macos {
        "vega-profile-preview-dock-lyra"
    } else {
        "vega-profile-preview-dock-gnome"
    });
    for _ in 0..4 {
        let icon = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        icon.add_css_class("vega-profile-preview-icon");
        dock.append(&icon);
    }
    if lyra {
        dock.set_halign(gtk::Align::Start);
        dock.set_valign(gtk::Align::Center);
        dock.set_margin_start(7);
    } else if ubuntu {
        dock.set_halign(gtk::Align::Start);
        dock.set_valign(gtk::Align::Fill);
        dock.set_margin_top(9);
        let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        dock.append(&spacer);
        let launcher = gtk::Label::new(Some("L"));
        dock.append(&launcher);
    } else {
        dock.set_halign(gtk::Align::Center);
        dock.set_valign(gtk::Align::End);
        dock.set_margin_bottom(7);
    }
    overlay.add_overlay(&dock);
    overlay.upcast()
}

fn windows_profile_preview(profile: DesktopProfile) -> gtk::Widget {
    let centered = profile == DesktopProfile::Windows11;
    let desktop = gtk::Box::new(gtk::Orientation::Vertical, 0);
    desktop.add_css_class("vega-profile-preview");
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&desktop));
    let panel = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    panel.add_css_class("vega-profile-preview-panel-gnome");
    panel.set_valign(gtk::Align::End);
    panel.set_halign(gtk::Align::Fill);
    panel.set_height_request(18);
    let icons = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    icons.set_halign(if centered {
        gtk::Align::Center
    } else {
        gtk::Align::Start
    });
    icons.set_hexpand(true);
    icons.set_margin_start(4);
    icons.append(&gtk::Label::new(Some("L")));
    for _ in 0..4 {
        let icon = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        icon.add_css_class("vega-profile-preview-icon");
        icons.append(&icon);
    }
    panel.append(&icons);
    overlay.add_overlay(&panel);
    let menu = gtk::Box::new(gtk::Orientation::Vertical, 6);
    menu.add_css_class("vega-profile-preview-start-menu");
    if centered {
        menu.add_css_class("windows11");
    }
    menu.set_size_request(110, 82);
    menu.set_halign(if centered {
        gtk::Align::Center
    } else {
        gtk::Align::Start
    });
    menu.set_valign(gtk::Align::End);
    menu.set_margin_bottom(20);
    menu.set_margin_start(if centered { 0 } else { 4 });
    let search = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    search.add_css_class("vega-profile-preview-search");
    menu.append(&search);
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if !centered {
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        for _ in 0..4 {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            row.add_css_class("vega-profile-preview-item");
            row.set_size_request(38, 4);
            list.append(&row);
        }
        body.append(&list);
    }
    let grid = gtk::Grid::builder()
        .row_spacing(4)
        .column_spacing(4)
        .build();
    for index in 0..6 {
        let icon = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        icon.add_css_class("vega-profile-preview-icon");
        grid.attach(&icon, index % 3, index / 3, 1, 1);
    }
    body.append(&grid);
    menu.append(&body);
    overlay.add_overlay(&menu);
    overlay.upcast()
}

/// Card grande de seleção de tema: janela em miniatura (clara ou escura) com
/// o nome do tema embaixo, igual ao seletor de estilo das Configurações do
/// GNOME — bem mais reconhecível que um combo de texto.
fn theme_card(
    dark: bool,
    label_text: String,
    group: Option<&gtk::ToggleButton>,
) -> gtk::ToggleButton {
    let preview = theme_preview(dark);

    let label = gtk::Label::builder()
        .label(&label_text)
        .css_classes(["heading"])
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.append(&preview);
    content.append(&label);

    let button = gtk::ToggleButton::builder()
        .child(&content)
        .css_classes(["flat", "vega-theme-card"])
        .height_request(150)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .valign(gtk::Align::Start)
        .build();
    if let Some(group) = group {
        button.set_group(Some(group));
    }
    button
}

/// Miniatura de janela (barra de título com três pontos + duas linhas de
/// conteúdo) só para dar contexto visual ao card — não é uma janela real.
fn theme_preview(dark: bool) -> gtk::Widget {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    header.add_css_class("vega-window-preview-header");
    header.set_valign(gtk::Align::Start);
    for _ in 0..3 {
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("vega-window-preview-dot");
        header.append(&dot);
    }

    let line_a = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line_a.add_css_class("vega-window-preview-line");
    line_a.set_size_request(96, -1);
    line_a.set_halign(gtk::Align::Start);

    let line_b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line_b.add_css_class("vega-window-preview-line");
    line_b.set_size_request(64, -1);
    line_b.set_halign(gtk::Align::Start);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.add_css_class("vega-window-preview-body");
    body.set_margin_top(12);
    body.set_margin_start(12);
    body.set_margin_end(12);
    body.set_vexpand(true);
    body.append(&line_a);
    body.append(&line_b);

    let window = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.add_css_class("vega-window-preview");
    window.add_css_class(if dark {
        "vega-window-preview-dark"
    } else {
        "vega-window-preview-light"
    });
    window.append(&header);
    window.append(&body);
    window.upcast()
}

#[cfg(test)]
mod desktop_tests {
    use super::*;
    use gtk::gio;

    #[test]
    #[ignore = "requires disposable GNOME compositor and packaged DING"]
    fn native_desktop_switch() {
        assert_eq!(
            std::env::var("DING_PRIVATE_NATIVE_TEST").as_deref(),
            Ok("1")
        );
        let home = std::env::var("HOME").unwrap();
        assert!(home.starts_with("/tmp/sheliak-pins-"));
        crate::i18n::init();
        adw::init().unwrap();
        let page = ScreenPage::new();
        fn find(widget: &gtk::Widget) -> Option<adw::SwitchRow> {
            if let Some(row) = widget.downcast_ref::<adw::SwitchRow>()
                && row.title() == gettext("Área de trabalho ativa")
            {
                return Some(row.clone());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(row) = find(&widget) {
                    return Some(row);
                }
                child = widget.next_sibling();
            }
            None
        }
        let row = find(&page.root).expect("desktop switch visible in profile page");
        assert!(row.is_sensitive());
        assert_eq!(Some(row.is_active()), crate::dock::desktop_icons_state());
        let fixture = std::path::Path::new(&home).join("Desktop/Fixture.txt");
        let before = std::fs::read(&fixture).unwrap();
        for enabled in [false, true] {
            row.set_active(enabled);
            assert_eq!(crate::dock::desktop_icons_state(), Some(enabled));
            assert_eq!(row.is_active(), enabled);
            assert_eq!(std::fs::read(&fixture).unwrap(), before);
        }
        // A global Shell switch must not be changed implicitly. The failed
        // enable must revert the row and expose the backend's actual state.
        let shell = gio::Settings::new("org.gnome.shell");
        shell.set_boolean("disable-user-extensions", true).unwrap();
        row.set_active(false);
        row.set_active(true);
        assert!(!row.is_active());
        assert!(shell.boolean("disable-user-extensions"));
        shell.set_boolean("disable-user-extensions", false).unwrap();
        row.set_active(true);
        assert!(row.is_active());
    }
}
