mod appearance;
mod application;
mod dock;
mod i18n;
mod model;
mod preferences;
mod profile_command;
mod screensaver;
mod tweaks;
mod ui;

fn main() -> gtk::glib::ExitCode {
    // Handle the local profile contract before GTK/GApplication or session UI.
    if let Some(code) = profile_command::run(std::env::args().skip(1).collect()) {
        return code.into();
    }
    i18n::init();
    application::run()
}
mod assistant;
