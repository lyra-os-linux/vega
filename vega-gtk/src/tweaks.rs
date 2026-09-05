use std::{cell::RefCell, ffi::OsStr};

use gtk::gio;

use crate::i18n::gettext;

#[derive(Clone, Copy)]
pub enum Page {
    Appearance,
    Fonts,
    Windows,
}

impl Page {
    fn key(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::Fonts => "fonts",
            Self::Windows => "windows",
        }
    }

    fn title(self) -> String {
        match self {
            Self::Appearance => gettext("Aparência"),
            Self::Fonts => gettext("Fontes"),
            Self::Windows => gettext("Janelas"),
        }
    }
}

thread_local! {
    static ACTIVE_SELECTION: RefCell<Option<gio::Subprocess>> = const { RefCell::new(None) };
}

/// A newer click supersedes any pending selection in the same native window.
/// Returns false when superseded so the old request cannot overwrite feedback.
pub async fn select_page(page: Page) -> Result<bool, String> {
    ACTIVE_SELECTION.with(|active| {
        if let Some(previous) = active.borrow_mut().take() {
            previous.force_exit();
        }
    });
    let error = || {
        gettext("O Ajustes foi aberto, mas não foi possível selecionar a aba {page}.")
            .replace("{page}", &page.title())
    };
    let process = gio::Subprocess::newv(
        &[
            OsStr::new("/usr/bin/python3"),
            OsStr::new("-c"),
            OsStr::new(include_str!("../resources/open-tweaks-page.py")),
            OsStr::new(page.key()),
        ],
        gio::SubprocessFlags::STDOUT_SILENCE | gio::SubprocessFlags::STDERR_SILENCE,
    )
    .map_err(|_| error())?;
    ACTIVE_SELECTION.with(|active| *active.borrow_mut() = Some(process.clone()));
    let result = process.wait_check_future().await;
    let is_current = ACTIVE_SELECTION.with(|active| {
        active
            .borrow_mut()
            .take_if(|current| current == &process)
            .is_some()
    });
    if !is_current {
        return Ok(false);
    }
    result.map(|_| true).map_err(|_| error())
}
