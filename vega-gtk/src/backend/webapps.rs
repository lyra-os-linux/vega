//! Webapps de bancos (vega#210) e de sites (vega#203): atalhos no menu de
//! aplicativos que abrem o site oficial no Chromium em modo app (janela
//! própria, sem a interface do navegador).
//!
//! Os endereços ficam fixados aqui; a interface só manda o id, e nenhum
//! endereço vindo de fora entra no atalho. O webapp de banco abre pelo
//! `vega-bank-webapp`, que liga o módulo de segurança (`warsaw.service`) antes
//! de abrir o site e o desliga quando o webapp fecha; o Vega instala o módulo
//! junto com o primeiro webapp de banco. O de site abre pelo `vega-webapp`,
//! sem o módulo.
//!
//! O atalho usa o ícone do site quando o Vega tem esse ícone; senão, o
//! genérico de navegador.

use std::path::{Path, PathBuf};

use gettextrs::gettext;
use gio::prelude::*;

/// Favoritos do dock (o Sheliak usa a mesma lista do GNOME Shell).
const SHELL_SCHEMA: &str = "org.gnome.shell";
const FAVORITES: &str = "favorite-apps";

pub struct Webapp {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    /// Banco: abre com o módulo de segurança, pelo `vega-bank-webapp`.
    pub bank: bool,
}

pub const WEBAPPS: &[Webapp] = &[
    Webapp {
        id: "banco-do-brasil",
        name: "Banco do Brasil",
        url: "https://www.bb.com.br/",
        bank: true,
    },
    Webapp {
        id: "caixa",
        name: "Caixa",
        url: "https://www.caixa.gov.br/",
        bank: true,
    },
    Webapp {
        id: "itau",
        name: "Itaú",
        url: "https://www.itau.com.br/",
        bank: true,
    },
    Webapp {
        id: "sicredi",
        name: "Sicredi",
        url: "https://www.sicredi.com.br/",
        bank: true,
    },
    Webapp {
        id: "safra",
        name: "Safra",
        url: "https://www.safra.com.br/",
        bank: true,
    },
    Webapp {
        id: "banese",
        name: "Banese",
        url: "https://www.banese.com.br/",
        bank: true,
    },
    Webapp {
        id: "bnb",
        name: "Banco do Nordeste",
        url: "https://www.bnb.gov.br/",
        bank: true,
    },
    Webapp {
        id: "brb",
        name: "BRB",
        url: "https://www.brb.com.br/",
        bank: true,
    },
    Webapp {
        id: "banco-da-amazonia",
        name: "Banco da Amazônia",
        url: "https://www.bancoamazonia.com.br/",
        bank: true,
    },
    Webapp {
        id: "bradesco",
        name: "Bradesco",
        url: "https://banco.bradesco/",
        bank: true,
    },
    Webapp {
        id: "banrisul",
        name: "Banrisul",
        url: "https://www.banrisul.com.br/",
        bank: true,
    },
    Webapp {
        id: "santander",
        name: "Santander",
        url: "https://www.santander.com.br/",
        bank: true,
    },
    Webapp {
        id: "nubank",
        name: "Nubank",
        url: "https://app.nubank.com.br/",
        bank: true,
    },
    Webapp {
        id: "sicoob",
        name: "Sicoob",
        url: "https://www.sicoob.com.br/",
        bank: true,
    },
    Webapp {
        id: "inter",
        name: "Inter",
        url: "https://inter.co/",
        bank: true,
    },
    Webapp {
        id: "c6",
        name: "C6 Bank",
        url: "https://www.c6bank.com.br/",
        bank: true,
    },
    Webapp {
        id: "btg",
        name: "BTG Pactual",
        url: "https://www.btgpactual.com/",
        bank: true,
    },
    // Sites (vega#203).
    Webapp {
        id: "outlook",
        name: "Outlook",
        url: "https://outlook.office.com/mail/",
        bank: false,
    },
    Webapp {
        id: "word",
        name: "Word",
        url: "https://www.office.com/launch/word",
        bank: false,
    },
    Webapp {
        id: "excel",
        name: "Excel",
        url: "https://www.office.com/launch/excel",
        bank: false,
    },
    Webapp {
        id: "teams",
        name: "Microsoft Teams",
        url: "https://teams.microsoft.com/",
        bank: false,
    },
    Webapp {
        id: "power-bi",
        name: "Power BI",
        url: "https://app.powerbi.com/",
        bank: false,
    },
    Webapp {
        id: "gmail",
        name: "Gmail",
        url: "https://mail.google.com/",
        bank: false,
    },
    Webapp {
        id: "google-drive",
        name: "Google Drive",
        url: "https://drive.google.com/",
        bank: false,
    },
    Webapp {
        id: "google-agenda",
        name: "Google Agenda",
        url: "https://calendar.google.com/",
        bank: false,
    },
    Webapp {
        id: "google-docs",
        name: "Google Docs",
        url: "https://docs.google.com/document/",
        bank: false,
    },
    Webapp {
        id: "jira",
        name: "Jira",
        url: "https://home.atlassian.com/",
        bank: false,
    },
    Webapp {
        id: "confluence",
        name: "Confluence",
        url: "https://home.atlassian.com/",
        bank: false,
    },
    Webapp {
        id: "asana",
        name: "Asana",
        url: "https://app.asana.com/",
        bank: false,
    },
    Webapp {
        id: "trello",
        name: "Trello",
        url: "https://trello.com/",
        bank: false,
    },
    Webapp {
        id: "monday",
        name: "monday.com",
        url: "https://auth.monday.com/login",
        bank: false,
    },
    Webapp {
        id: "notion",
        name: "Notion",
        url: "https://www.notion.so/",
        bank: false,
    },
    Webapp {
        id: "canva",
        name: "Canva",
        url: "https://www.canva.com/",
        bank: false,
    },
    Webapp {
        id: "figma",
        name: "Figma",
        url: "https://www.figma.com/files/",
        bank: false,
    },
    Webapp {
        id: "whatsapp",
        name: "WhatsApp",
        url: "https://web.whatsapp.com/",
        bank: false,
    },
];

/// Prefixo do atalho, do ícone e da classe da janela (`StartupWMClass`).
fn prefix(app: &Webapp) -> &'static str {
    if app.bank {
        "lyra-banco-"
    } else {
        "lyra-webapp-"
    }
}

/// Pasta do perfil do Chromium, a mesma que o lançador usa.
fn profile_path(data: &Path, app: &Webapp) -> PathBuf {
    let dir = if app.bank {
        "lyra-bancos"
    } else {
        "lyra-webapps"
    };
    data.join(dir).join(app.id)
}

/// Ícone do site (PNG 128 px, do site oficial, do Wikimedia Commons ou do
/// Simple Icons), o mesmo do cartão.
/// Sem ícone, o atalho usa o genérico de navegador.
fn icon(app: &Webapp) -> Option<&'static [u8]> {
    if !app.bank {
        return site_icon(app.id);
    }
    match app.id {
        "banco-da-amazonia" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-banco-da-amazonia.png"
        )),
        "banco-do-brasil" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-banco-do-brasil.png"
        )),
        "banese" => Some(include_bytes!("../../ui/img/popular/lyra-banco-banese.png")),
        "banrisul" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-banrisul.png"
        )),
        "bnb" => Some(include_bytes!("../../ui/img/popular/lyra-banco-bnb.png")),
        "bradesco" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-bradesco.png"
        )),
        "brb" => Some(include_bytes!("../../ui/img/popular/lyra-banco-brb.png")),
        "btg" => Some(include_bytes!("../../ui/img/popular/lyra-banco-btg.png")),
        "c6" => Some(include_bytes!("../../ui/img/popular/lyra-banco-c6.png")),
        "caixa" => Some(include_bytes!("../../ui/img/popular/lyra-banco-caixa.png")),
        "inter" => Some(include_bytes!("../../ui/img/popular/lyra-banco-inter.png")),
        "itau" => Some(include_bytes!("../../ui/img/popular/lyra-banco-itau.png")),
        "nubank" => Some(include_bytes!("../../ui/img/popular/lyra-banco-nubank.png")),
        "safra" => Some(include_bytes!("../../ui/img/popular/lyra-banco-safra.png")),
        "santander" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-santander.png"
        )),
        "sicoob" => Some(include_bytes!("../../ui/img/popular/lyra-banco-sicoob.png")),
        "sicredi" => Some(include_bytes!(
            "../../ui/img/popular/lyra-banco-sicredi.png"
        )),
        _ => None,
    }
}

fn site_icon(id: &str) -> Option<&'static [u8]> {
    match id {
        "asana" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-asana.png")),
        "canva" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-canva.png")),
        "confluence" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-confluence.png"
        )),
        "excel" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-excel.png")),
        "figma" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-figma.png")),
        "gmail" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-gmail.png")),
        "google-agenda" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-google-agenda.png"
        )),
        "google-docs" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-google-docs.png"
        )),
        "google-drive" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-google-drive.png"
        )),
        "jira" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-jira.png")),
        "monday" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-monday.png"
        )),
        "notion" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-notion.png"
        )),
        "outlook" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-outlook.png"
        )),
        "power-bi" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-power-bi.png"
        )),
        "teams" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-teams.png")),
        "trello" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-trello.png"
        )),
        "whatsapp" => Some(include_bytes!(
            "../../ui/img/popular/lyra-webapp-whatsapp.png"
        )),
        "word" => Some(include_bytes!("../../ui/img/popular/lyra-webapp-word.png")),
        _ => None,
    }
}

fn icon_path(data: &Path, app: &Webapp) -> PathBuf {
    data.join("icons/hicolor/128x128/apps")
        .join(format!("{}{}.png", prefix(app), app.id))
}

fn webapp(id: &str) -> Option<&'static Webapp> {
    WEBAPPS.iter().find(|app| app.id == id)
}

fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

fn desktop_path(data: &Path, app: &Webapp) -> PathBuf {
    data.join("applications")
        .join(format!("{}{}.desktop", prefix(app), app.id))
}

/// Conteúdo do atalho do menu de aplicativos.
/// `icon_file`: caminho completo do PNG do site. O GNOME Shell só relê o
/// tema de ícones quando a pasta do tema muda, não quando entra um arquivo
/// novo em `128x128/apps`; com o caminho completo, o ícone aparece na hora.
pub fn desktop_entry(app: &Webapp, icon_file: Option<&Path>) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={name}\n\
         Comment={comment}\n\
         Exec={launcher} {id} {url}\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories={categories}\n\
         StartupNotify=true\n\
         StartupWMClass={prefix}{id}\n\
         X-Lyra-Vega-Webapp={id}\n",
        name = app.name,
        comment = if app.bank {
            gettext("Site oficial do banco, numa janela própria")
        } else {
            gettext("Site oficial, numa janela própria")
        },
        launcher = if app.bank {
            "vega-bank-webapp"
        } else {
            "vega-webapp"
        },
        categories = if app.bank {
            "Network;Finance;"
        } else {
            "Network;WebBrowser;"
        },
        prefix = prefix(app),
        url = app.url,
        icon = icon_file.map_or_else(
            || "web-browser".to_owned(),
            |path| path.display().to_string()
        ),
        id = app.id,
    )
}

/// Lista de favoritos com `entry` no fim; `None` se ele já estava lá.
pub fn with_favorite(favorites: &[String], entry: &str) -> Option<Vec<String>> {
    if favorites.iter().any(|item| item == entry) {
        return None;
    }
    let mut list = favorites.to_vec();
    list.push(entry.to_owned());
    Some(list)
}

/// Lista de favoritos sem `entry`; `None` se ele não estava lá.
pub fn without_favorite(favorites: &[String], entry: &str) -> Option<Vec<String>> {
    if !favorites.iter().any(|item| item == entry) {
        return None;
    }
    Some(
        favorites
            .iter()
            .filter(|item| *item != entry)
            .cloned()
            .collect(),
    )
}

/// Fixa ou solta o webapp no dock. Sem o esquema do GNOME Shell (outra
/// sessão), não faz nada: o atalho continua no menu de aplicativos.
fn set_favorite(app: &Webapp, pinned: bool) {
    let Some(source) = gio::SettingsSchemaSource::default() else {
        return;
    };
    if source.lookup(SHELL_SCHEMA, true).is_none() {
        return;
    }
    let settings = gio::Settings::new(SHELL_SCHEMA);
    let entry = format!("{}{}.desktop", prefix(app), app.id);
    let current: Vec<String> = settings
        .strv(FAVORITES)
        .iter()
        .map(|item| item.to_string())
        .collect();
    let next = if pinned {
        with_favorite(&current, &entry)
    } else {
        without_favorite(&current, &entry)
    };
    if let Some(list) = next {
        let list: Vec<&str> = list.iter().map(String::as_str).collect();
        let _ = settings.set_strv(FAVORITES, list.as_slice());
        gio::Settings::sync();
    }
}

fn refresh_menu(data: &Path) {
    let mut command = std::process::Command::new("update-desktop-database");
    command.arg(data.join("applications"));
    crate::webkit_env::clean_command(&mut command);
    let _ = command.status();
}

pub fn installed_in(data: &Path) -> Vec<String> {
    WEBAPPS
        .iter()
        .filter(|app| desktop_path(data, app).is_file())
        .map(|app| app.id.to_owned())
        .collect()
}

#[tauri::command]
pub fn webapps_installed() -> Vec<String> {
    data_dir()
        .map(|data| installed_in(&data))
        .unwrap_or_default()
}

#[tauri::command]
pub fn webapp_install(id: String) -> Result<(), String> {
    let app = webapp(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let path = desktop_path(&data, app);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    if let Some(bytes) = icon(app) {
        let icon = icon_path(&data, app);
        if let Some(dir) = icon.parent() {
            std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        }
        std::fs::write(&icon, bytes).map_err(|error| error.to_string())?;
    }
    let icon_file = icon(app).map(|_| icon_path(&data, app));
    std::fs::write(&path, desktop_entry(app, icon_file.as_deref()))
        .map_err(|error| error.to_string())?;
    refresh_menu(&data);
    // Já entra no dock, como o mantenedor pediu em 15/11/2026.
    set_favorite(app, true);
    Ok(())
}

/// Apaga o perfil do Chromium do webapp (logins, cookies, cache). Recusa
/// com o webapp aberto: o Chromium mantém o `SingletonLock` no perfil
/// enquanto roda e regravaria os dados.
pub fn remove_profile(profile: &Path) -> Result<(), String> {
    if profile.join("SingletonLock").symlink_metadata().is_ok() {
        return Err(gettext(
            "Feche o web app antes de apagar os dados de login.",
        ));
    }
    match std::fs::remove_dir_all(profile) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// `delete_data`: apaga também o perfil do Chromium, com os logins. Sem
/// ele, reinstalar o webapp volta já logado.
#[tauri::command]
pub fn webapp_remove(id: String, delete_data: Option<bool>) -> Result<(), String> {
    let app = webapp(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    // Antes do atalho: se o webapp estiver aberto, nada muda.
    if delete_data == Some(true) {
        remove_profile(&profile_path(&data, app))?;
    }
    for path in [desktop_path(&data, app), icon_path(&data, app)] {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    // Sem o atalho, o favorito viraria um ícone quebrado no dock.
    set_favorite(app, false);
    refresh_menu(&data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_urls_are_https() {
        let mut ids: Vec<_> = WEBAPPS.iter().map(|app| app.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), WEBAPPS.len());
        for app in WEBAPPS {
            assert!(app.url.starts_with("https://"), "{}", app.id);
            assert!(
                app.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{}",
                app.id
            );
            assert!(!app.url.contains(char::is_whitespace), "{}", app.id);
        }
    }

    #[test]
    fn shortcut_runs_the_launcher_with_the_pinned_url() {
        let bb = desktop_entry(webapp("banco-do-brasil").unwrap(), None);
        assert!(bb.contains(
            "Exec=vega-bank-webapp banco-do-brasil https://www.bb.com.br/
"
        ));
    }

    #[test]
    fn shortcut_uses_the_bank_icon_when_there_is_one() {
        let file = Path::new(
            "/home/ana/.local/share/icons/hicolor/128x128/apps/lyra-banco-banco-do-brasil.png",
        );
        let bb = desktop_entry(webapp("banco-do-brasil").unwrap(), Some(file));
        assert!(bb.contains(&format!("Icon={}\n", file.display())));
        assert!(desktop_entry(webapp("caixa").unwrap(), None).contains("Icon=web-browser\n"));
        assert!(bb.contains("StartupWMClass=lyra-banco-banco-do-brasil\n"));
        for app in WEBAPPS {
            assert!(icon(app).is_some(), "{} sem ícone", app.id);
        }
    }

    #[test]
    fn favorites_keep_the_user_list() {
        let list = vec!["firefox.desktop".to_owned(), "vega.desktop".to_owned()];
        let pinned = with_favorite(&list, "lyra-banco-itau.desktop").unwrap();
        assert_eq!(
            pinned,
            ["firefox.desktop", "vega.desktop", "lyra-banco-itau.desktop"]
        );
        assert!(with_favorite(&pinned, "lyra-banco-itau.desktop").is_none());
        assert_eq!(
            without_favorite(&pinned, "lyra-banco-itau.desktop").unwrap(),
            list
        );
        assert!(without_favorite(&list, "lyra-banco-itau.desktop").is_none());
    }

    #[test]
    fn installed_follows_the_desktop_files() {
        let root = std::env::temp_dir().join(format!("vega-webapps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("applications")).unwrap();
        assert!(installed_in(&root).is_empty());
        std::fs::write(root.join("applications/lyra-banco-caixa.desktop"), "").unwrap();
        assert_eq!(installed_in(&root), ["caixa"]);
        std::fs::write(root.join("applications/lyra-webapp-gmail.desktop"), "").unwrap();
        assert_eq!(installed_in(&root), ["caixa", "gmail"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn site_shortcut_uses_the_site_launcher_without_the_bank_module() {
        let gmail = desktop_entry(webapp("gmail").unwrap(), None);
        assert!(gmail.contains("Exec=vega-webapp gmail https://mail.google.com/\n"));
        assert!(gmail.contains("StartupWMClass=lyra-webapp-gmail\n"));
        assert!(gmail.contains("Categories=Network;WebBrowser;\n"));
        assert!(!gmail.contains("vega-bank-webapp"));
        let data = Path::new("/home/ana/.local/share");
        assert_eq!(
            profile_path(data, webapp("gmail").unwrap()),
            data.join("lyra-webapps/gmail")
        );
        assert_eq!(
            profile_path(data, webapp("caixa").unwrap()),
            data.join("lyra-bancos/caixa")
        );
    }

    #[test]
    fn profile_is_kept_while_the_webapp_is_open() {
        let root = std::env::temp_dir().join(format!("vega-webapp-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Default")).unwrap();
        std::os::unix::fs::symlink("host-123", root.join("SingletonLock")).unwrap();
        assert!(remove_profile(&root).is_err());
        assert!(root.join("Default").is_dir());
        std::fs::remove_file(root.join("SingletonLock")).unwrap();
        remove_profile(&root).unwrap();
        assert!(!root.exists());
        // Perfil que nunca existiu (webapp nunca aberto) não é erro.
        remove_profile(&root).unwrap();
    }
}
