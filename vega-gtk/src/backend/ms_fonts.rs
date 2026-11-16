//! Fontes TrueType da Microsoft ("core fonts for the web": Arial, Times New
//! Roman, Verdana, Courier New...). O Leap 16.1 não traz o `fetchmsttfonts`
//! do Tumbleweed, então o Vega faz o mesmo na conta do usuário, sem senha:
//! baixa os 11 instaladores do SourceForge, confere o SHA-512 de cada um (a
//! lista `corefonts.sha512` do fetchmsttfonts) e extrai só os `.ttf` com o
//! `cabextract` em `~/.local/share/fonts/microsoft-core`. A licença da
//! Microsoft permite instalar, não redistribuir: por isso o Lyra não empacota
//! as fontes.

use std::path::{Path, PathBuf};
use std::process::Command;

use gettextrs::gettext;
use sha2::{Digest, Sha512};

use super::receita_apps::download;

pub const ID: &str = "ms-core-fonts";
const BASE_URL: &str = "https://downloads.sourceforge.net/project/corefonts/the%20fonts/final/";
const DIR: &str = "fonts/microsoft-core";

const INSTALLERS: &[(&str, &str)] = &[
    (
        "andale32.exe",
        "7099c389126d2af146ff082a840c77787f28e59cc8b5fd4147b9a45a34c649bc69ba6ec7c338703950389fce3ac3a7f118f503b7434c7ceb54c89d6210508bd4",
    ),
    (
        "arial32.exe",
        "299cb200f72f146fedfb2bb1895c36f5234aa5abd084e7c962ec7b14f087c1f582b51f5fdcb72e48b806ee02661c34766ca8a03bb0f2fadf662aaf46cc5f0e48",
    ),
    (
        "arialb32.exe",
        "8a947764f2c725f2c506c12ca6f77142402174a0fadeeb72b211931136d82b541bb279b415d74032bc51b00efac9516e5cf68d8d4f172ff8549398ca28722c3a",
    ),
    (
        "comic32.exe",
        "6ad11015d568d58f6e4375e04411dd702b5c4e454b82998b5044cd8352dae27fea1df6469a05067ccc8c14f4ba87ec99e8329c104551dde1eb48be8ce2b78708",
    ),
    (
        "courie32.exe",
        "7e2425d578dd626cc2b418efec72c53e35392c60610cf5e4a99c614fa86cb673167fbcc1691fe59b99bcd19e8f261321be9ea7e30fb84c80ce0a312fd47045c5",
    ),
    (
        "georgi32.exe",
        "33e623179af8a418845fed64f7b176a1e721ee4e0ece29d3135d01b81be4238454dd4eb3d2166e8587bb7cced0c258a294eb55e637855f818deb28853f006ef4",
    ),
    (
        "impact32.exe",
        "6f9318806efbd3b3727e71170cc3ab378d6151887abcb7fad01bd4cb45c2e127f8aea81a3a99bc77c2ce2424fb1acbc230ffc6c23e390c8800710b45b1367c3b",
    ),
    (
        "times32.exe",
        "6035b971290cd0795389c2e8a0f02d2f079468e197a1db791189519e12aaf3fcdaf6e8842a7eeca386ad7f48f85a0b07cc5480b98ee06b0529688fd5550d3ee2",
    ),
    (
        "trebuc32.exe",
        "4ba8afac0a419e30286f5b209b9fabf41aa480ceabb1bf3141deb775793510db826ba915b67fdfe3423d227c3e640bf7d682b26c0ce90f3c0e2cdf8d4831537e",
    ),
    (
        "verdan32.exe",
        "96749fd17e4efeccdd467cb8f42ce30fc3a9e86eceef8b9c0934a4693d12944b54864006c5852c0d641a5c7b250553e5c6c21d716e446d02ef392299c7833d58",
    ),
    (
        "webdin32.exe",
        "2784ce1cee66f0f00f77d84d7aaa52a44d017245e0269558729003205c10a5e5368f9909beee733e77e202730b3cee2b2357fdbdb597a0023c6412e050b3a3ad",
    ),
];

fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

fn fonts_dir() -> Option<PathBuf> {
    data_home().map(|data| data.join(DIR))
}

fn has_fonts(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf"))
        })
    })
}

fn sha512_hex(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha512::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn refresh_cache(dir: &Path) {
    let _ = Command::new("fc-cache").arg("-f").arg(dir).status();
}

#[tauri::command]
pub fn ms_fonts_installed() -> Vec<String> {
    match fonts_dir() {
        Some(dir) if has_fonts(&dir) => vec![ID.to_owned()],
        _ => Vec::new(),
    }
}

fn install(dir: &Path, work: &Path) -> Result<(), String> {
    for (file, sha512) in INSTALLERS {
        let target = work.join(file);
        download(&format!("{BASE_URL}{file}"), &target)?;
        if sha512_hex(&target)? != *sha512 {
            return Err(gettext(
                "O arquivo {file} baixado não confere com o original. Nada foi instalado.",
            )
            .replace("{file}", file));
        }
    }
    let staging = work.join("ttf");
    std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    for (file, _) in INSTALLERS {
        let status = Command::new("cabextract")
            .args(["-q", "-L", "-F", "*.ttf", "-d"])
            .arg(&staging)
            .arg(work.join(file))
            .status()
            .map_err(|_| gettext("O cabextract não está instalado."))?;
        if !status.success() {
            return Err(gettext("Não foi possível extrair {file}.").replace("{file}", file));
        }
    }
    if !has_fonts(&staging) {
        return Err(gettext("Nenhuma fonte foi extraída."));
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let _ = std::fs::remove_dir_all(dir);
    std::fs::rename(&staging, dir).map_err(|error| error.to_string())?;
    refresh_cache(dir);
    Ok(())
}

/// Baixa, confere e instala as fontes; nada muda se um arquivo não conferir.
#[tauri::command]
pub async fn ms_fonts_install() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = fonts_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
        let data = data_home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
        std::fs::create_dir_all(&data).map_err(|error| error.to_string())?;
        let work = tempfile_dir(&data)?;
        let result = install(&dir, &work);
        let _ = std::fs::remove_dir_all(&work);
        result
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Pasta de trabalho no mesmo sistema de arquivos do destino (rename atômico).
fn tempfile_dir(data: &Path) -> Result<PathBuf, String> {
    let work = data.join(format!(".vega-ms-fonts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|error| error.to_string())?;
    Ok(work)
}

#[tauri::command]
pub fn ms_fonts_remove() -> Result<(), String> {
    let dir = fonts_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    if let Some(parent) = dir.parent() {
        refresh_cache(parent);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_installer_has_a_sha512() {
        assert_eq!(INSTALLERS.len(), 11);
        for (file, sha512) in INSTALLERS {
            assert!(file.ends_with("32.exe"), "{file}");
            assert_eq!(sha512.len(), 128, "{file}");
            assert!(sha512.chars().all(|c| c.is_ascii_hexdigit()), "{file}");
        }
    }

    #[test]
    fn sha512_matches_a_known_value() {
        let dir = std::env::temp_dir().join(format!("vega-ms-fonts-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("abc");
        std::fs::write(&file, b"abc").unwrap();
        assert!(sha512_hex(&file).unwrap().starts_with("ddaf35a193617aba"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
