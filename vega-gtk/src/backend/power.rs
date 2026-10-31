//! Reinício pedido pelo próprio Vega (aviso "Reinicie o computador para
//! concluir").
//!
//! Algumas mudanças só valem depois de reiniciar: o parâmetro
//! `ia32_emulation=1` (apps de 32 bits do Windows) e o Flatpak recém-habilitado
//! (o GNOME só lê a pasta dos apps do Flathub no login). A interface guarda o
//! `boot_id` em que o reinício foi pedido; quando ele muda, o aviso some.
//! Reiniciar usa o logind pela sessão do usuário (`systemctl reboot`), que o
//! polkit libera para quem está na sessão local ativa.

use std::process::Command;

use gettextrs::gettext;

/// Identificador desta inicialização do kernel (muda a cada boot).
pub fn boot_id_from(text: &str) -> Option<String> {
    let id = text.trim();
    (id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')).then(|| id.to_owned())
}

#[tauri::command]
pub fn system_boot_id() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .and_then(|text| boot_id_from(&text))
}

#[tauri::command]
pub fn system_reboot() -> Result<(), String> {
    let mut command = Command::new("systemctl");
    command.arg("reboot");
    crate::webkit_env::clean_command(&mut command);
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if detail.is_empty() {
        gettext("Não foi possível reiniciar o computador.")
    } else {
        detail
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_id_accepts_only_the_kernel_uuid() {
        assert_eq!(
            boot_id_from("9c1f4d2e-3a5b-4c6d-8e7f-0123456789ab\n").as_deref(),
            Some("9c1f4d2e-3a5b-4c6d-8e7f-0123456789ab")
        );
        assert_eq!(boot_id_from(""), None);
        assert_eq!(boot_id_from("não é um id"), None);
    }
}
