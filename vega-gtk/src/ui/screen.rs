use crate::i18n::gettext;
use adw::prelude::*;

use super::{DockPage, MenuPage, ScreensaverPage};
use crate::appearance::Theme;

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
        let (appearance, profile) =
            appearance_pages(&overview.menu, &overview.dock, crate::dock::is_installed());
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
        true,
        None,
    );
    let vanilla_profile = profile_card(
        &gettext("Gnome Vanila"),
        &gettext("Usa a experiência padrão do GNOME."),
        false,
        Some(&lyra_profile),
    );
    lyra_profile.set_sensitive(sheliak_available);
    lyra_profile.set_active(sheliak_available && crate::dock::is_enabled());
    vanilla_profile.set_active(!lyra_profile.is_active());

    let lyra_menu_tab = menu_tab.clone();
    let lyra_dock_tab = dock_tab.clone();
    lyra_profile.connect_toggled(move |button| {
        if button.is_active() && crate::dock::set_enabled(true).is_ok() {
            lyra_menu_tab.set_sensitive(true);
            lyra_dock_tab.set_sensitive(true);
        }
    });
    let vanilla_menu_tab = menu_tab.clone();
    let vanilla_dock_tab = dock_tab.clone();
    vanilla_profile.connect_toggled(move |button| {
        if button.is_active() && crate::dock::set_enabled(false).is_ok() {
            vanilla_menu_tab.set_sensitive(false);
            vanilla_dock_tab.set_sensitive(false);
        }
    });

    let profiles = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    profiles.set_homogeneous(true);
    profiles.set_valign(gtk::Align::Start);
    profiles.append(&lyra_profile);
    profiles.append(&vanilla_profile);

    let profile_group = adw::PreferencesGroup::builder()
        .title(gettext("Perfil da área de trabalho"))
        .valign(gtk::Align::Start)
        .build();
    profile_group.add(&profiles);

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
    lyra: bool,
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
        .css_classes(["dim-label"])
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.append(&profile_preview(lyra));
    content.append(&title);
    content.append(&description);
    let button = gtk::ToggleButton::builder()
        .child(&content)
        .css_classes(["flat", "vega-profile-card"])
        .build();
    if let Some(group) = group {
        button.set_group(Some(group));
    }
    button
}

/// Ilustração compacta do desktop de cada perfil. É construída com widgets e
/// CSS (sem imagem externa): Lyra tem painel flutuante e dock lateral; GNOME
/// Vanilla tem painel colado ao topo e dash central inferior.
fn profile_preview(lyra: bool) -> gtk::Widget {
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
    overlay.add_overlay(&panel);

    let dock = gtk::Box::new(
        if lyra {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        },
        4,
    );
    dock.add_css_class("vega-profile-preview-dock");
    dock.add_css_class(if lyra {
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
    } else {
        dock.set_halign(gtk::Align::Center);
        dock.set_valign(gtk::Align::End);
        dock.set_margin_bottom(7);
    }
    overlay.add_overlay(&dock);
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
