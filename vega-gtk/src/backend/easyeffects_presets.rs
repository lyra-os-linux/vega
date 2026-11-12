//! Perfis do Easy Effects instalados pelo Vega na conta do usuário (vega#198).
//!
//! O Easy Effects 8 lê os presets de entrada (microfone) em
//! `<dados do app>/easyeffects/input`. No Flatpak, os dados do app ficam em
//! `~/.var/app/com.github.wwmm.easyeffects/data`; no pacote, em
//! `~/.local/share`. O Vega grava o preset nas instalações que existirem e o
//! usuário escolhe o perfil no próprio Easy Effects.

use std::path::{Path, PathBuf};

use gettextrs::gettext;

const FLATPAK_ID: &str = "com.github.wwmm.easyeffects";

/// Perfil de entrada: id do cartão, nome do arquivo e conteúdo.
pub struct Preset {
    pub id: &'static str,
    pub file: &'static str,
    pub json: &'static str,
}

/// "Voz para gravação": passa-alta em 80 Hz, redução de ruído (RNNoise) com
/// detecção de voz, compressor, equalizador de voz e limitador em -1 dB.
pub const PRESETS: &[Preset] = &[Preset {
    id: "easyeffects-voice",
    file: "Voz para gravação.json",
    json: include_str!("presets/voz-para-gravacao.json"),
}];

fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

/// Pastas de presets de entrada das instalações do Easy Effects que existem.
pub fn input_dirs(home: &Path, data: &Path, flatpak: bool, package: bool) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if flatpak {
        dirs.push(
            home.join(".var/app")
                .join(FLATPAK_ID)
                .join("data/easyeffects/input"),
        );
    }
    if package {
        dirs.push(data.join("easyeffects/input"));
    }
    dirs
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn installed_dirs() -> Option<Vec<PathBuf>> {
    let home = home()?;
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"));
    let flatpak = Path::new("/var/lib/flatpak/app").join(FLATPAK_ID).is_dir()
        || home
            .join(".local/share/flatpak/app")
            .join(FLATPAK_ID)
            .is_dir();
    let package = Path::new("/usr/bin/easyeffects").is_file();
    Some(input_dirs(&home, &data, flatpak, package))
}

#[tauri::command]
pub fn easyeffects_presets_installed() -> Vec<String> {
    let dirs = installed_dirs().unwrap_or_default();
    PRESETS
        .iter()
        .filter(|preset| !dirs.is_empty() && dirs.iter().all(|dir| dir.join(preset.file).is_file()))
        .map(|preset| preset.id.to_owned())
        .collect()
}

#[tauri::command]
pub fn easyeffects_preset_install(id: String) -> Result<(), String> {
    let preset = preset(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    let dirs = installed_dirs().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    if dirs.is_empty() {
        return Err(gettext(
            "Instale o Easy Effects antes de instalar o perfil.",
        ));
    }
    for dir in dirs {
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        std::fs::write(dir.join(preset.file), preset.json).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn easyeffects_preset_remove(id: String) -> Result<(), String> {
    let preset = preset(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    for dir in installed_dirs().unwrap_or_default() {
        if let Err(error) = std::fs::remove_file(dir.join(preset.file))
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_preset_is_a_valid_input_chain() {
        let json: serde_json::Value = serde_json::from_str(PRESETS[0].json).unwrap();
        let order = json["input"]["plugins_order"].as_array().unwrap();
        for plugin in order {
            let name = plugin.as_str().unwrap();
            assert!(json["input"][name].is_object(), "{name} sem configuração");
        }
        assert_eq!(json["input"]["limiter#0"]["threshold"], -1.0);
    }

    #[test]
    fn writes_where_each_installation_reads() {
        let home = Path::new("/home/ana");
        let data = Path::new("/home/ana/.local/share");
        assert_eq!(
            input_dirs(home, data, true, false),
            [Path::new(
                "/home/ana/.var/app/com.github.wwmm.easyeffects/data/easyeffects/input"
            )]
        );
        assert_eq!(
            input_dirs(home, data, false, true),
            [Path::new("/home/ana/.local/share/easyeffects/input")]
        );
        assert!(input_dirs(home, data, false, false).is_empty());
    }
}
