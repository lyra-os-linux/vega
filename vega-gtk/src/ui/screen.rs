use crate::i18n::gettext;
use adw::prelude::*;

use super::{DockPage, MenuPage, ScreensaverPage, WallpaperPage};
use crate::appearance::Theme;

/// Reúne tudo relacionado a "tela": aparência, bloqueio de tela, papel de
/// parede, o menu e o dock do Sheliak (quando instalado) —
/// uma única entrada de navegação com abas internas, como o módulo Software.
#[derive(Clone)]
pub struct ScreenPage {
    pub root: gtk::Widget,
    pub screensaver: ScreensaverPage,
    pub wallpaper: WallpaperPage,
    pub menu: MenuPage,
    pub dock: DockPage,
}

impl ScreenPage {
    pub fn new() -> Self {
        let screensaver = ScreensaverPage::new();
        let wallpaper = WallpaperPage::new();
        let menu = MenuPage::new();
        let dock = DockPage::new();
        let appearance = appearance_page();

        let appearance_tab = tab_button(&gettext("Aparência"));
        let wallpaper_tab = tab_button(&gettext("Papel de Parede"));
        let screensaver_tab = tab_button(&gettext("Proteção de Tela"));
        let menu_tab = tab_button(&gettext("Menu"));
        let dock_tab = tab_button(&gettext("Dock"));
        appearance_tab.set_active(true);
        wallpaper_tab.set_group(Some(&appearance_tab));
        screensaver_tab.set_group(Some(&appearance_tab));
        menu_tab.set_group(Some(&appearance_tab));
        dock_tab.set_group(Some(&appearance_tab));
        // Menu e Dock são partes da extensão Sheliak: as abas só aparecem
        // quando a extensão está instalada, seguindo o mesmo padrão de
        // disponibilidade condicional usado pelo resto do Vega (ex.:
        // schema_available()).
        menu_tab.set_visible(crate::dock::is_installed());
        dock_tab.set_visible(crate::dock::is_installed());

        let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        tabs.add_css_class("module-tabs");
        tabs.append(&appearance_tab);
        tabs.append(&wallpaper_tab);
        tabs.append(&screensaver_tab);
        tabs.append(&menu_tab);
        tabs.append(&dock_tab);

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .vexpand(true)
            .build();
        stack.add_named(&appearance, Some("appearance"));
        stack.add_named(&wallpaper.root, Some("wallpaper"));
        stack.add_named(&screensaver.root, Some("screensaver"));
        stack.add_named(&menu.root, Some("menu"));
        stack.add_named(&dock.root, Some("dock"));
        stack.set_visible_child_name("appearance");

        let appearance_stack = stack.clone();
        appearance_tab.connect_clicked(move |button| {
            if button.is_active() {
                appearance_stack.set_visible_child_name("appearance");
            }
        });
        let screensaver_stack = stack.clone();
        screensaver_tab.connect_clicked(move |button| {
            if button.is_active() {
                screensaver_stack.set_visible_child_name("screensaver");
            }
        });
        let wallpaper_stack = stack.clone();
        wallpaper_tab.connect_clicked(move |button| {
            if button.is_active() {
                wallpaper_stack.set_visible_child_name("wallpaper");
            }
        });
        let menu_stack = stack.clone();
        menu_tab.connect_clicked(move |button| {
            if button.is_active() {
                menu_stack.set_visible_child_name("menu");
            }
        });
        let dock_stack = stack.clone();
        dock_tab.connect_clicked(move |button| {
            if button.is_active() {
                dock_stack.set_visible_child_name("dock");
            }
        });

        let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
        content.add_css_class("content-page");
        content.append(
            &gtk::Label::builder()
                .label(gettext("Personalização"))
                .xalign(0.0)
                .css_classes(["title-1"])
                .build(),
        );
        content.append(
            &gtk::Label::builder()
                .label(gettext("Aparência, bloqueio de tela e papel de parede"))
                .xalign(0.0)
                .css_classes(["dim-label"])
                .build(),
        );
        content.append(&tabs);
        content.append(&stack);

        Self {
            root: content.upcast(),
            screensaver,
            wallpaper,
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

fn tab_button(label: &str) -> gtk::ToggleButton {
    gtk::ToggleButton::builder()
        .label(label)
        .css_classes(["flat", "module-tab"])
        .build()
}

/// O tema escreve direto em `org.gnome.desktop.interface` (veja
/// `crate::appearance`): não é preferência do Vega, é a mesma
/// configuração do painel Aparência do GNOME — muda o Shell, o Nautilus e
/// qualquer app libadwaita em execução, não só a janela do Vega.
fn appearance_page() -> gtk::Widget {
    let unavailable = !crate::appearance::schema_available();

    let theme_group = adw::PreferencesGroup::builder()
        .title(gettext("Tema"))
        .build();

    let cards = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    cards.set_homogeneous(true);
    cards.set_hexpand(true);
    cards.set_margin_top(4);
    cards.set_margin_bottom(4);

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
            apply_enterprise_wallpaper();
            crate::appearance::apply_icon_theme();
        }
    });
    dark_card.connect_toggled(|button| {
        if button.is_active() {
            crate::appearance::apply_theme(Theme::Dark);
            apply_enterprise_wallpaper();
            crate::appearance::apply_icon_theme();
        }
    });

    cards.append(&light_card);
    cards.append(&dark_card);
    theme_group.add(&cards);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    if unavailable {
        content.append(
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
    content.append(&theme_group);

    gtk::ScrolledWindow::builder()
        .child(&content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
        .upcast()
}

/// Ao trocar o card de tema, também força o papel de parede padrão do
/// Lyra OS (par `os.png`/`os-light.png`, entrada "Lyra OS" no XML de
/// gnome-background-properties) — o GNOME já troca sozinho entre eles depois
/// disso, via `picture-uri-dark`, então basta garantir que as duas URIs
/// estejam apontando pra esse par.
///
/// O nome mudou de "Lyra Enterprise" pra "Lyra OS" quando o Lyra-Theme foi
/// renomeado (Lyra-Theme@47d0ff4); a busca exata evita casar com as
/// variantes de humor adicionais ("Lyra OS — Nebula" etc.) que o Lyra-Theme
/// também registra.
fn apply_enterprise_wallpaper() {
    if let Some(entry) = crate::wallpaper::list_wallpapers()
        .into_iter()
        .find(|entry| entry.name.eq_ignore_ascii_case("Lyra OS"))
    {
        let _ = crate::wallpaper::apply(&entry);
    }
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
        .height_request(250)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .valign(gtk::Align::Center)
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
