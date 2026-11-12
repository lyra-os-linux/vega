//! Codificação de vídeo por hardware para o OBS (vega#196).
//!
//! Lê os fabricantes das placas em `/sys/class/drm` e se o módulo da NVIDIA
//! está carregado. Não roda nada: o objetivo é só dizer, no kit do OBS, qual
//! codificador escolher ou qual driver falta.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

const NVIDIA: &str = "0x10de";
const AMD: &str = "0x1002";
const INTEL: &str = "0x8086";

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Encoders {
    /// NVIDIA com o driver proprietário carregado.
    pub nvenc: bool,
    /// NVIDIA sem o driver proprietário: o OBS codifica pelo processador.
    pub nvidia_without_driver: bool,
    /// AMD ou Intel, pelo VA-API do Mesa.
    pub vaapi: bool,
}

pub fn classify(vendors: &BTreeSet<String>, nvidia_driver: bool) -> Encoders {
    let nvidia = vendors.contains(NVIDIA);
    Encoders {
        nvenc: nvidia && nvidia_driver,
        nvidia_without_driver: nvidia && !nvidia_driver,
        vaapi: vendors.contains(AMD) || vendors.contains(INTEL),
    }
}

fn vendors(drm: &Path) -> BTreeSet<String> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("device/vendor")).ok())
        .map(|vendor| vendor.trim().to_ascii_lowercase())
        .collect()
}

#[tauri::command]
pub fn obs_encoders() -> Encoders {
    classify(
        &vendors(Path::new("/sys/class/drm")),
        Path::new("/sys/module/nvidia").is_dir(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(vendors: &[&str]) -> BTreeSet<String> {
        vendors.iter().map(|vendor| (*vendor).to_owned()).collect()
    }

    #[test]
    fn nvidia_needs_the_proprietary_driver() {
        assert_eq!(
            classify(&set(&[NVIDIA]), true),
            Encoders {
                nvenc: true,
                ..Default::default()
            }
        );
        assert_eq!(
            classify(&set(&[NVIDIA]), false),
            Encoders {
                nvidia_without_driver: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn hybrid_laptop_offers_both() {
        let found = classify(&set(&[INTEL, NVIDIA]), true);
        assert!(found.nvenc && found.vaapi);
        assert_eq!(classify(&set(&[]), false), Encoders::default());
        assert!(classify(&set(&[AMD]), false).vaapi);
    }
}
