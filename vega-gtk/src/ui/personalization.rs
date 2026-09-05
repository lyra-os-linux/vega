use adw::prelude::*;
use gtk::{gio, glib};

use crate::tweaks;

use crate::i18n::gettext;

const SETTINGS: &[&str] = &["org.gnome.Settings.desktop"];
const BACKGROUND: &[&str] = &["gnome-background-panel.desktop"];
const TWEAKS: &[&str] = &["org.gnome.tweaks.desktop", "gnome-tweak-tool.desktop"];
const EXTENSIONS: &[&str] = &[
    "org.gnome.Extensions.desktop",
    "org.gnome.Shell.Extensions.desktop",
];

#[derive(Clone, Copy)]
enum Destination {
    Page(&'static str),
    App(&'static [&'static str]),
    Tweaks(tweaks::Page),
}

#[derive(Clone, Copy)]
enum Summary {
    Theme,
    Accent,
    Setting(&'static str),
    Profile,
    Default,
}

struct Card {
    button: gtk::Button,
    value: gtk::Label,
    destination: Destination,
    summary: Summary,
}

pub struct PersonalizationOverview {
    pub root: gtk::Widget,
    pub menu: gtk::Button,
    pub dock: gtk::Button,
}

impl PersonalizationOverview {
    pub fn new(stack: &gtk::Stack) -> Self {
        let status = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .css_classes(["error"])
            .build();
        let refresh = gtk::Button::builder()
            .child(
                &adw::ButtonContent::builder()
                    .icon_name("view-refresh-symbolic")
                    .label(gettext("Atualizar"))
                    .build(),
            )
            .valign(gtk::Align::Center)
            .build();
        let heading = gtk::Box::new(gtk::Orientation::Vertical, 4);
        heading.set_hexpand(true);
        heading.append(&label(&gettext("Personalização"), "title-1"));
        heading.append(&label(
            &gettext("Ajuste a aparência e o comportamento do GNOME"),
            "dim-label",
        ));
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header.append(&heading);
        header.append(&refresh);

        let grid = card_grid();
        let mut cards = Vec::new();
        for (title, description, icon, destination, summary) in [
            (
                gettext("Tema"),
                gettext("Aparência clara ou escura para a área de trabalho e os aplicativos"),
                "preferences-desktop-theme-symbolic",
                Destination::Page("appearance"),
                Summary::Theme,
            ),
            (
                gettext("Cores"),
                gettext("Cor de destaque dos botões, seleções e elementos da interface"),
                "applications-graphics-symbolic",
                Destination::App(BACKGROUND),
                Summary::Accent,
            ),
            (
                gettext("Ícones"),
                gettext("Conjunto de ícones dos aplicativos e do sistema"),
                "view-grid-symbolic",
                Destination::Tweaks(tweaks::Page::Appearance),
                Summary::Setting("icon-theme"),
            ),
            (
                gettext("Fontes"),
                gettext("Família, tamanho e renderização dos textos"),
                "preferences-desktop-font-symbolic",
                Destination::Tweaks(tweaks::Page::Fonts),
                Summary::Setting("font-name"),
            ),
            (
                gettext("Cursores"),
                gettext("Tema e tamanho do ponteiro do mouse"),
                "input-mouse-symbolic",
                Destination::Tweaks(tweaks::Page::Appearance),
                Summary::Setting("cursor-theme"),
            ),
            (
                gettext("Janelas"),
                gettext("Botões da barra de título, foco e comportamento das janelas"),
                "focus-windows-symbolic",
                Destination::Tweaks(tweaks::Page::Windows),
                Summary::Default,
            ),
            (
                gettext("Papel de Parede"),
                gettext("Imagem e comportamento do plano de fundo"),
                "preferences-desktop-wallpaper-symbolic",
                Destination::App(BACKGROUND),
                Summary::Default,
            ),
            (
                gettext("Tela de bloqueio"),
                gettext("Bloqueio automático e tempo de inatividade da tela"),
                "system-lock-screen-symbolic",
                Destination::Page("screensaver"),
                Summary::Default,
            ),
        ] {
            let card = Card::new(
                &title,
                &description,
                icon,
                destination,
                summary,
                stack,
                &status,
            );
            grid.insert(&card.button, -1);
            cards.push(card);
        }

        let lyra_grid = card_grid();
        for (title, description, icon, page, summary) in [
            (
                gettext("Perfil da área de trabalho"),
                gettext("Escolha entre o ambiente Lyra e a experiência padrão do GNOME"),
                "preferences-desktop-display-symbolic",
                "profile",
                Summary::Profile,
            ),
            (
                gettext("Menu"),
                gettext("Painel, relógio e menu de aplicativos do Lyra"),
                "open-menu-symbolic",
                "menu",
                Summary::Default,
            ),
            (
                gettext("Dock"),
                gettext("Posição, ícones e comportamento do dock do Lyra"),
                "view-app-grid-symbolic",
                "dock",
                Summary::Default,
            ),
        ] {
            let card = Card::new(
                &title,
                &description,
                icon,
                Destination::Page(page),
                summary,
                stack,
                &status,
            );
            lyra_grid.insert(&card.button, -1);
            cards.push(card);
        }
        let menu = cards[9].button.clone();
        let dock = cards[10].button.clone();

        let apps_grid = card_grid();
        for (title, description, icon, ids) in [
            (
                gettext("Configurações do GNOME"),
                gettext("Preferências de tela, teclado, mouse e outros ajustes da sessão"),
                "org.gnome.Settings",
                SETTINGS,
            ),
            (
                gettext("Ajustes do GNOME"),
                gettext("Opções adicionais de aparência, fontes e janelas"),
                "org.gnome.tweaks",
                TWEAKS,
            ),
            (
                gettext("Extensões do GNOME"),
                gettext("Gerencie as extensões e suas preferências"),
                "org.gnome.Extensions",
                EXTENSIONS,
            ),
        ] {
            let card = Card::new(
                &title,
                &description,
                icon,
                Destination::App(ids),
                Summary::Default,
                stack,
                &status,
            );
            apps_grid.insert(&card.button, -1);
            cards.push(card);
        }

        let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content.append(&header);
        content.append(&status);
        content.append(&grid);
        content.append(&label(&gettext("Ambiente Lyra"), "title-2"));
        content.append(&lyra_grid);
        content.append(&label(&gettext("Aplicativos do GNOME"), "title-2"));
        content.append(&apps_grid);
        let root = gtk::ScrolledWindow::builder()
            .child(&content)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let update = std::rc::Rc::new(move || {
            let apps = gio::AppInfo::all();
            for card in &cards {
                card.refresh(&apps);
            }
        });
        update();
        let refresh_update = update.clone();
        refresh.connect_clicked(move |_| {
            status.set_visible(false);
            refresh_update();
        });
        // Refresh when reopening the overview, including after a profile change.
        root.connect_map(move |_| update());

        Self {
            root: root.upcast(),
            menu,
            dock,
        }
    }
}

impl Card {
    fn new(
        title: &str,
        description: &str,
        icon: &str,
        destination: Destination,
        summary: Summary,
        stack: &gtk::Stack,
        status: &gtk::Label,
    ) -> Self {
        let image = gtk::Image::builder()
            .icon_name(icon)
            .pixel_size(28)
            .valign(gtk::Align::Center)
            .css_classes(["personalization-icon"])
            .build();
        let value = label("", "personalization-value");
        value.set_ellipsize(gtk::pango::EllipsizeMode::End);
        value.set_wrap(false);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 5);
        text.set_hexpand(true);
        text.append(&label(title, "heading"));
        let description = label(description, "dim-label");
        description.set_max_width_chars(32);
        text.append(&description);
        text.append(&value);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        content.append(&image);
        content.append(&text);
        content.append(&gtk::Image::from_icon_name("go-next-symbolic"));
        let button = gtk::Button::builder()
            .child(&content)
            .hexpand(true)
            .css_classes(["flat", "personalization-card"])
            .build();
        button.update_property(&[gtk::accessible::Property::Label(title)]);
        let stack = stack.downgrade();
        let status = status.clone();
        let title = title.to_owned();
        button.connect_clicked(move |_| match destination {
            Destination::Page(page) => {
                if let Some(stack) = stack.upgrade() {
                    stack.set_visible_child_name(page);
                }
            }
            Destination::App(_) | Destination::Tweaks(_) => {
                let ids = match destination {
                    Destination::App(ids) => ids,
                    _ => TWEAKS,
                };
                let result = find_app(&gio::AppInfo::all(), ids)
                    .ok_or_else(|| gettext("Aplicativo não instalado"))
                    .and_then(|app| {
                        let context = gtk::gdk::Display::default()
                            .map(|display| display.app_launch_context());
                        if matches!(destination, Destination::Tweaks(_))
                            && let Some(context) = &context
                        {
                            // Enable accessibility for this launched app only.
                            context.setenv("GTK_A11Y", "atspi");
                        }
                        app.launch(&[], context.as_ref())
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => {
                        status.set_visible(false);
                        if let Destination::Tweaks(page) = destination {
                            let status = status.clone();
                            glib::MainContext::default().spawn_local(async move {
                                match tweaks::select_page(page).await {
                                    Ok(true) => status.set_visible(false),
                                    Ok(false) => {}
                                    Err(error) => {
                                        status.set_label(&error);
                                        status.set_visible(true);
                                    }
                                }
                            });
                        }
                    }
                    Err(error) => {
                        status.set_label(
                            &gettext("Não foi possível abrir {name}: {error}")
                                .replace("{name}", &title)
                                .replace("{error}", &error),
                        );
                        status.set_visible(true);
                    }
                }
            }
        });
        Self {
            button,
            value,
            destination,
            summary,
        }
    }

    fn refresh(&self, apps: &[gio::AppInfo]) {
        let (available, fallback) = match self.destination {
            Destination::App(ids) => app_availability(apps, ids),
            Destination::Tweaks(_) => app_availability(apps, TWEAKS),
            Destination::Page("menu" | "dock") => {
                let enabled = crate::dock::is_installed() && crate::dock::is_enabled();
                (
                    enabled,
                    if enabled {
                        gettext("Configurar")
                    } else {
                        gettext("Requer o perfil Lyra com Sheliak ativo")
                    },
                )
            }
            Destination::Page(_) => (true, gettext("Configurar")),
        };
        let value = if available {
            match self.summary {
                Summary::Theme => match crate::appearance::current_theme() {
                    crate::appearance::Theme::Light => gettext("Claro"),
                    crate::appearance::Theme::Dark => gettext("Escuro"),
                    crate::appearance::Theme::System => gettext("Padrão do sistema"),
                },
                Summary::Accent => accent_color().unwrap_or_else(|| fallback.clone()),
                Summary::Setting(key) => interface_setting(key).unwrap_or_else(|| fallback.clone()),
                Summary::Profile => {
                    if crate::dock::is_installed() && crate::dock::is_enabled() {
                        gettext("Lyra")
                    } else {
                        gettext("GNOME padrão")
                    }
                }
                Summary::Default => fallback.clone(),
            }
        } else {
            fallback.clone()
        };
        self.button.set_sensitive(available);
        self.button.set_tooltip_text(Some(&fallback));
        self.value.set_label(&value);
    }
}

fn app_availability(apps: &[gio::AppInfo], ids: &[&str]) -> (bool, String) {
    match find_app(apps, ids) {
        Some(app) => (
            true,
            gettext("Abrir {name}").replace("{name}", &app.display_name()),
        ),
        None => (false, gettext("Aplicativo não instalado")),
    }
}

fn find_app(apps: &[gio::AppInfo], ids: &[&str]) -> Option<gio::AppInfo> {
    ids.iter().find_map(|id| {
        apps.iter()
            .find(|app| {
                app.id().as_deref() == Some(*id)
                // GNOME Shell ships an Exec=false desktop file for portal
                // identification, even when the Extensions app is absent.
                && app.executable().file_name() != Some(std::ffi::OsStr::new("false"))
            })
            .cloned()
    })
}

fn interface_setting(key: &str) -> Option<String> {
    let schema =
        gio::SettingsSchemaSource::default()?.lookup("org.gnome.desktop.interface", true)?;
    if !schema.has_key(key) {
        return None;
    }
    Some(
        gio::Settings::new_full(&schema, gio::SettingsBackend::NONE, None)
            .string(key)
            .into(),
    )
}

fn accent_color() -> Option<String> {
    Some(match interface_setting("accent-color")?.as_str() {
        "blue" => gettext("Azul"),
        "teal" => gettext("Verde-azulado"),
        "green" => gettext("Verde"),
        "yellow" => gettext("Amarelo"),
        "orange" => gettext("Laranja"),
        "red" => gettext("Vermelho"),
        "pink" => gettext("Rosa"),
        "purple" => gettext("Roxo"),
        "slate" => gettext("Cinza"),
        _ => return None,
    })
}

fn label(text: &str, class: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .css_classes([class])
        .build()
}

fn card_grid() -> gtk::FlowBox {
    gtk::FlowBox::builder()
        .column_spacing(16)
        .row_spacing(16)
        .min_children_per_line(1)
        .max_children_per_line(2)
        .homogeneous(true)
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["personalization-grid"])
        .build()
}
