//! Cartão "Driver NVIDIA" (topo de Populares). O Vega não gerencia mais a
//! pilha NVIDIA (remove-nvidia-integration, 31/10/2026); o cartão só segue o
//! guia do fórum: repositório oficial da NVIDIA para o Leap 16.1, com a chave
//! conferida, e o metapacote certo para a placa. A instalação passa pelo
//! vegad como qualquer pacote (polkit e snapshot). Aqui só se detecta:
//! geração da placa, Secure Boot, a integração antiga (lyra-nvidia) e o que
//! já está instalado.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

pub const G07: &str = "nvidia-open-driver-G07-signed-kmp-meta";
pub const G06: &str = "nvidia-driver-G06-kmp-meta";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NvidiaPlan {
    /// "g07", "g06", "legacy" (sem driver para o Leap 16) ou "none".
    generation: String,
    /// Nome da placa como o lspci mostra, quando há.
    model: String,
    /// Metapacote a instalar ("" em legacy/none).
    package: String,
    secure_boot: bool,
    /// Sobrou a integração antiga do Vega: a migração é pelo guia.
    lyra_nvidia: bool,
    /// Metapacote do guia já instalado.
    installed: bool,
}

/// Geração do driver pela família do chip (TU/GA/AD/GH/GB: G07 aberto;
/// GM/GP/GV: G06; GK, GF e anteriores: sem driver no Leap 16).
fn generation(chip: &str) -> &'static str {
    match chip.get(..2) {
        Some("TU" | "GA" | "AD" | "GH" | "GB") => "g07",
        Some("GM" | "GP" | "GV") => "g06",
        _ => "legacy",
    }
}

/// Primeira placa NVIDIA de vídeo do `lspci -nn`: (chip, modelo).
fn parse_lspci(out: &str) -> Option<(String, String)> {
    out.lines()
        .filter(|line| {
            line.contains("NVIDIA") && (line.contains("VGA") || line.contains("3D controller"))
        })
        .find_map(|line| {
            let after = line.split("NVIDIA Corporation").nth(1)?.trim();
            let chip = after.split_whitespace().next().filter(|word| {
                let bytes = word.as_bytes();
                bytes.len() >= 4
                    && bytes[..2].iter().all(u8::is_ascii_uppercase)
                    && bytes[2].is_ascii_digit()
            })?;
            let model = after
                .split_once('[')
                .and_then(|(_, rest)| rest.split_once(']'))
                .map(|(name, _)| name.to_owned())
                .unwrap_or_else(|| chip.to_owned());
            Some((chip.to_owned(), model))
        })
}

/// Secure Boot pela variável EFI (4 bytes de atributos + 1 de valor).
fn secure_boot() -> bool {
    std::fs::read_dir("/sys/firmware/efi/efivars")
        .ok()
        .and_then(|entries| {
            entries.flatten().find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("SecureBoot-")
            })
        })
        .and_then(|entry| std::fs::read(entry.path()).ok())
        .is_some_and(|data| data.last() == Some(&1))
}

fn rpm_installed(name: &str) -> bool {
    Command::new("rpm")
        .args(["-q", "--quiet", name])
        .status()
        .is_ok_and(|status| status.success())
}

#[tauri::command]
pub fn nvidia_plan() -> NvidiaPlan {
    let lspci = Command::new("lspci")
        .arg("-nn")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    let (generation, model) = match parse_lspci(&lspci) {
        Some((chip, model)) => (generation(&chip), model),
        None => ("none", String::new()),
    };
    let package = match generation {
        "g07" => G07,
        "g06" => G06,
        _ => "",
    };
    NvidiaPlan {
        generation: generation.to_owned(),
        model,
        package: package.to_owned(),
        secure_boot: secure_boot(),
        lyra_nvidia: rpm_installed("lyra-nvidia")
            || Path::new("/etc/zypp/repos.d/lyra-nvidia.repo").exists(),
        installed: rpm_installed(G07) || rpm_installed(G06),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generations_follow_the_forum_guide() {
        let cases = [
            (
                "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation TU106 [GeForce RTX 2060] [10de:1f08] (rev a1)",
                "g07",
                "GeForce RTX 2060",
            ),
            (
                "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation AD107M [GeForce RTX 4060 Max-Q / Mobile] [10de:28e0] (rev a1)",
                "g07",
                "GeForce RTX 4060 Max-Q / Mobile",
            ),
            (
                "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation TU117 [GeForce GTX 1650] [10de:1f82] (rev a1)",
                "g07",
                "GeForce GTX 1650",
            ),
            (
                "01:00.0 3D controller [0302]: NVIDIA Corporation GP107M [GeForce GTX 1050 Mobile] [10de:1c8d] (rev a1)",
                "g06",
                "GeForce GTX 1050 Mobile",
            ),
            (
                "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GM107 [GeForce GTX 750 Ti] [10de:1380] (rev a2)",
                "g06",
                "GeForce GTX 750 Ti",
            ),
            (
                "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GK104 [GeForce GTX 680] [10de:1180] (rev a1)",
                "legacy",
                "GeForce GTX 680",
            ),
        ];
        for (line, want, model) in cases {
            let (chip, got_model) = parse_lspci(line).expect(line);
            assert_eq!(generation(&chip), want, "{line}");
            assert_eq!(got_model, model);
        }
    }

    #[test]
    fn ignores_audio_and_other_vendors() {
        let out = "00:02.0 VGA compatible controller [0300]: Intel Corporation Alder Lake-P GT2 [8086:46a6]\n\
                   01:00.1 Audio device [0403]: NVIDIA Corporation TU106 High Definition Audio Controller [10de:10f9]\n";
        assert_eq!(parse_lspci(out), None);
    }
}
