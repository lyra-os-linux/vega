//! Contorno do crash do WebKitGTK 2.52 ao encerrar (vega#166).
//!
//! O renderer DMA-BUF do WebKit guarda o dispositivo GBM num singleton
//! estático (`WebCore::GBMDevice`). No `exit()` do WebKitWebProcess, o
//! destrutor dele corre contra o desmonte do Mesa e do glvnd e corrompe o
//! heap ("corrupted double-linked list"): cerca de metade dos fechamentos do
//! Vega gerava um coredump. Com o renderer desligado, 0 em 20 ciclos; o
//! compositing acelerado continua ligado. Retirar quando o WebKit corrigir.

use gio::prelude::*;
use std::ffi::OsStr;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

pub const DISABLE_DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// Se a variável foi definida pelo Vega (e não herdada do usuário).
static SET_BY_VEGA: AtomicBool = AtomicBool::new(false);

/// Desliga o renderer DMA-BUF do WebKit. Chamar no início do `main`, antes
/// de qualquer thread. Um valor já definido pelo usuário é respeitado.
pub fn apply() {
    if should_set(std::env::var_os(DISABLE_DMABUF).as_deref()) {
        // SAFETY: chamado no início do main, com uma única thread.
        unsafe { std::env::set_var(DISABLE_DMABUF, "1") };
        SET_BY_VEGA.store(true, Ordering::Relaxed);
    }
}

fn should_set(existing: Option<&OsStr>) -> bool {
    existing.is_none()
}

/// Apps gráficos abertos pelo Vega não herdam o contorno.
pub fn clean_command(command: &mut Command) -> &mut Command {
    strip_command(command, SET_BY_VEGA.load(Ordering::Relaxed))
}

fn strip_command(command: &mut Command, set_by_vega: bool) -> &mut Command {
    if set_by_vega {
        command.env_remove(DISABLE_DMABUF);
    }
    command
}

/// Mesmo cuidado para apps lançados pelo GIO.
pub fn clean_launch_context(context: &gio::AppLaunchContext) {
    if SET_BY_VEGA.load(Ordering::Relaxed) {
        context.unsetenv(DISABLE_DMABUF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_only_when_the_user_did_not() {
        assert!(should_set(None));
        assert!(!should_set(Some(OsStr::new("0"))));
        assert!(!should_set(Some(OsStr::new("1"))));
    }

    fn removed(command: &Command) -> bool {
        command
            .get_envs()
            .any(|(key, value)| key == DISABLE_DMABUF && value.is_none())
    }

    #[test]
    fn children_lose_the_workaround_set_by_vega() {
        let mut command = Command::new("true");
        strip_command(&mut command, true);
        assert!(removed(&command));
    }

    #[test]
    fn children_keep_a_value_the_user_set() {
        let mut command = Command::new("true");
        strip_command(&mut command, false);
        assert!(!removed(&command));
    }

    #[test]
    fn launch_context_unsets_the_workaround() {
        let context = gio::AppLaunchContext::new();
        context.setenv(DISABLE_DMABUF, "1");
        SET_BY_VEGA.store(true, Ordering::Relaxed);
        clean_launch_context(&context);
        let left = context.environment().iter().any(|entry| {
            entry
                .to_str()
                .is_some_and(|e| e.starts_with(DISABLE_DMABUF))
        });
        assert!(!left);
    }
}
