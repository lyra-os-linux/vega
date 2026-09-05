mod appearance;
mod application;
mod dock;
mod i18n;
mod model;
mod preferences;
mod screensaver;
mod tweaks;
mod ui;

fn main() -> gtk::glib::ExitCode {
    i18n::init();
    application::run()
}
mod assistant;
