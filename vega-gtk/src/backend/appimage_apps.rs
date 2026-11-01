//! AppImages oficiais instalados na conta do usuário (Vega → Software).
//!
//! Para aplicativos que só publicam AppImage nas releases do GitHub e se
//! atualizam sozinhos (como o ArtCraft Launcher, que lança versão a cada
//! poucos dias), uma versão fixada no Vega envelheceria em dias. Então o Vega
//! pega a release mais recente pela API do GitHub e confere o AppImage contra
//! o arquivo de checksums publicado na mesma release; sem o checksum, ou com
//! ele diferente, nada é instalado. O arquivo vai para `~/Applications`, onde
//! o próprio aplicativo se atualiza, com atalho no menu e ícone na conta do
//! usuário. Só os aplicativos de `APPS` são aceitos.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gettextrs::gettext;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use super::software::TransactionEvent;

/// AppImage oficial publicado nas releases de um repositório do GitHub.
#[derive(Debug, Clone, Copy)]
pub struct App {
    pub id: &'static str,
    pub name: &'static str,
    /// `dono/repositório` no GitHub.
    pub repo: &'static str,
    /// Começo do nome do AppImage (`<prefixo><versão>-linux-<arch>.AppImage`).
    pub prefix: &'static str,
    /// Arquivo de checksums da release (formato do sha256sum).
    pub sums: &'static str,
    /// Ícone do aplicativo no repositório, no ref da release.
    pub icon_path: &'static str,
    pub comment: &'static str,
}

pub const APPS: &[App] = &[App {
    id: "artcraft-launcher",
    name: "ArtCraft Launcher",
    repo: "storytold/craft-launcher",
    prefix: "artcraft-launcher-",
    sums: "SHA256SUMS.txt",
    icon_path: "assets/app-icon/hicolor/256x256/apps/ai.storyteller.artcraft-launcher.png",
    comment: "Install and update the Craft apps",
}];

pub fn app(id: &str) -> Option<&'static App> {
    APPS.iter().find(|app| app.id == id)
}

/// Ids próprios, acima dos do npm.
static NEXT_ID: AtomicU32 = AtomicU32::new(0xE000_0000);

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".local/share")))
}

fn image_path(home: &Path, app: &App) -> PathBuf {
    home.join("Applications")
        .join(format!("{}.AppImage", app.id))
}

fn desktop_path(data: &Path, app: &App) -> PathBuf {
    data.join(format!("applications/lyra-appimage-{}.desktop", app.id))
}

fn icon_path(data: &Path, app: &App) -> PathBuf {
    data.join(format!(
        "icons/hicolor/256x256/apps/lyra-appimage-{}.png",
        app.id
    ))
}

/// Arquitetura no nome dos AppImages (`x86_64`, `aarch64`).
fn arch() -> &'static str {
    std::env::consts::ARCH
}

/// Nome do AppImage desta arquitetura entre os assets da release.
pub fn pick_asset<'a>(app: &App, assets: &'a [String], arch: &str) -> Option<&'a String> {
    let suffix = format!("-linux-{arch}.AppImage");
    assets.iter().find(|name| {
        name.strip_prefix(app.prefix)
            .and_then(|rest| rest.strip_suffix(suffix.as_str()))
            .is_some_and(|version| {
                !version.is_empty() && version.chars().all(|c| c.is_ascii_digit() || c == '.')
            })
    })
}

/// Checksum de `file` no formato do sha256sum (`<hash>  <arquivo>`).
pub fn checksum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == file && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn desktop_entry(app: &App, image: &Path, icon: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\nExec=\"{image}\" %U\nIcon={icon}\nTerminal=false\nStartupNotify=true\n",
        name = app.name,
        comment = app.comment,
        image = image.display(),
    )
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .https_only(true)
        .user_agent("lyra-vega")
        .timeout(std::time::Duration::from_secs(900))
        .build()
        .map_err(|error| error.to_string())
}

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.bytes())
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}

fn install(app: &App, report: &dyn Fn(u32, String)) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let client = client()?;
    report(5, gettext("Procurando a versão mais recente…"));
    let release: Release = serde_json::from_slice(&get(
        &client,
        &format!("https://api.github.com/repos/{}/releases/latest", app.repo),
    )?)
    .map_err(|error| error.to_string())?;
    let names: Vec<String> = release
        .assets
        .iter()
        .map(|asset| asset.name.clone())
        .collect();
    let file = pick_asset(app, &names, arch())
        .ok_or_else(|| gettext("A versão mais recente não tem AppImage para este computador."))?
        .clone();
    let url_of = |name: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| asset.browser_download_url.clone())
    };
    let sums_url = url_of(app.sums)
        .ok_or_else(|| gettext("A release não publica checksums; nada foi instalado."))?;
    let sums = String::from_utf8_lossy(&get(&client, &sums_url)?).into_owned();
    let expected = checksum_for(&sums, &file)
        .ok_or_else(|| gettext("A release não publica checksums; nada foi instalado."))?;
    report(20, gettext("Baixando {file}…").replace("{file}", &file));
    let image = get(&client, &url_of(&file).unwrap_or_default())?;
    if sha256_hex(&image) != expected {
        return Err(gettext(
            "O AppImage baixado não confere com o checksum da release; nada foi instalado.",
        ));
    }
    report(80, gettext("Instalando {file}…").replace("{file}", &file));
    let target = image_path(&home, app);
    std::fs::create_dir_all(target.parent().unwrap_or(&home)).map_err(|error| error.to_string())?;
    let partial = target.with_extension("AppImage.part");
    {
        let mut out = std::fs::File::create(&partial).map_err(|error| error.to_string())?;
        out.write_all(&image).map_err(|error| error.to_string())?;
    }
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    std::fs::rename(&partial, &target).map_err(|error| error.to_string())?;
    // O ícone é só cosmético: sem ele, o atalho usa um ícone genérico.
    let icon_file = icon_path(&data, app);
    let icon = match get(
        &client,
        &format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            app.repo, release.tag_name, app.icon_path
        ),
    ) {
        Ok(bytes) if bytes.starts_with(b"\x89PNG") => {
            let _ = std::fs::create_dir_all(icon_file.parent().unwrap_or(&data));
            if std::fs::write(&icon_file, bytes).is_ok() {
                icon_file.display().to_string()
            } else {
                "application-x-executable".into()
            }
        }
        _ => "application-x-executable".into(),
    };
    let entry = desktop_path(&data, app);
    std::fs::create_dir_all(entry.parent().unwrap_or(&data)).map_err(|error| error.to_string())?;
    std::fs::write(&entry, desktop_entry(app, &target, &icon)).map_err(|error| error.to_string())
}

fn remove(app: &App) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    for path in [
        image_path(&home, app),
        desktop_path(&data, app),
        icon_path(&data, app),
    ] {
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.to_string());
        }
    }
    Ok(())
}

fn refresh_menu() {
    if let Some(data) = data_dir() {
        let mut command = std::process::Command::new("update-desktop-database");
        command.arg(data.join("applications"));
        crate::webkit_env::clean_command(&mut command);
        let _ = command.status();
    }
}

#[tauri::command]
pub fn appimage_installed() -> Vec<String> {
    let Some(home) = home() else {
        return Vec::new();
    };
    APPS.iter()
        .filter(|app| image_path(&home, app).is_file())
        .map(|app| app.id.to_owned())
        .collect()
}

fn start(app_handle: AppHandle, id: String, installing: bool) -> Result<u32, String> {
    let app = app(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        let emit = |event: TransactionEvent| {
            let _ = app_handle.emit("software-transaction", event);
        };
        let report = |percent: u32, message: String| {
            emit(TransactionEvent::Progress {
                id: tx,
                percent,
                message,
            })
        };
        let result = if installing {
            install(app, &report)
        } else {
            remove(app)
        };
        refresh_menu();
        emit(TransactionEvent::Finished {
            id: tx,
            success: result.is_ok(),
            message: result.err().unwrap_or_else(|| gettext("Concluído")),
        });
    });
    Ok(tx)
}

#[tauri::command]
pub fn appimage_install(app: AppHandle, id: String) -> Result<u32, String> {
    start(app, id, true)
}

#[tauri::command]
pub fn appimage_remove(app: AppHandle, id: String) -> Result<u32, String> {
    start(app, id, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAUNCHER: &App = &APPS[0];

    #[test]
    fn picks_only_the_appimage_for_this_arch() {
        let assets: Vec<String> = [
            "artcraft-launcher-0.12.0-linux-aarch64.AppImage",
            "artcraft-launcher-0.12.0-linux-x86_64.AppImage.zsync",
            "artcraft-launcher-0.12.0-linux-x86_64.AppImage",
            "artcraft-launcher-0.12.0-linux-x86_64.rpm",
            "SHA256SUMS.txt",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            pick_asset(LAUNCHER, &assets, "x86_64").map(String::as_str),
            Some("artcraft-launcher-0.12.0-linux-x86_64.AppImage")
        );
        assert_eq!(
            pick_asset(LAUNCHER, &assets, "aarch64").map(String::as_str),
            Some("artcraft-launcher-0.12.0-linux-aarch64.AppImage")
        );
        assert_eq!(pick_asset(LAUNCHER, &assets, "riscv64"), None);
        let tricky = vec!["artcraft-launcher-evil;rm-linux-x86_64.AppImage".to_owned()];
        assert_eq!(pick_asset(LAUNCHER, &tricky, "x86_64"), None);
    }

    #[test]
    fn checksum_is_read_from_sha256sums() {
        let sums = "257b5be8792311e559012dca524e2447c68aa3e753bd5a2df20f40fac610d3a4  artcraft-launcher-0.12.0-linux-aarch64.AppImage\nC70F0FB8BDC20602BC7907526BB8342AB8E48D6BE1BD57C252F3E4DBCA444F4A *artcraft-launcher-0.12.0-linux-x86_64.AppImage\n";
        assert_eq!(
            checksum_for(sums, "artcraft-launcher-0.12.0-linux-x86_64.AppImage").as_deref(),
            Some("c70f0fb8bdc20602bc7907526bb8342ab8e48d6be1bd57c252f3e4dbca444f4a")
        );
        assert_eq!(checksum_for(sums, "outro.AppImage"), None);
        assert_eq!(checksum_for("abc  x.AppImage", "x.AppImage"), None);
    }

    #[test]
    fn desktop_entry_runs_the_appimage_in_home() {
        let entry = desktop_entry(
            LAUNCHER,
            Path::new("/home/u/Applications/artcraft-launcher.AppImage"),
            "/i.png",
        );
        assert!(entry.contains("Name=ArtCraft Launcher\n"));
        assert!(entry.contains("Exec=\"/home/u/Applications/artcraft-launcher.AppImage\" %U\n"));
        assert!(entry.contains("Icon=/i.png\n"));
    }

    #[test]
    fn apps_point_at_github_repositories() {
        for app in APPS {
            assert_eq!(app.repo.matches('/').count(), 1, "{}", app.id);
            assert!(!app.icon_path.starts_with('/'), "{}", app.id);
        }
    }
}
