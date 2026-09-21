use super::{DockPage, MenuPage, ScreensaverPage};
use crate::dock::SettingsContext;
use adw::prelude::*;
use std::{cell::Cell, rc::Rc};

#[test]
#[ignore = "requires a GTK display"]
fn native_settings_context() {
    adw::init().unwrap();
    let active = SettingsContext {
        dock: true,
        panel: true,
        menus: true,
        search: true,
        animations: true,
        ..Default::default()
    };
    let dock = DockPage::new();
    let mut saved = dock.selected();
    saved.edge_margin = 17;
    saved.extend_to_edges = true;
    saved.show_running = false;
    saved.minimize_animation = "fade".into();
    dock.show(&saved);
    dock.set_context(active);
    assert!(!dock.edge_margin.is_sensitive());
    assert!(!dock.running_apps_position.is_sensitive());
    assert!(dock.minimize_animation.is_sensitive());
    assert_eq!(dock.selected(), saved);
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    dock.connect_changed(move |_| count.set(count.get() + 1));
    dock.set_context(SettingsContext {
        animations: true,
        ..Default::default()
    });
    assert!(!dock.icon_size.is_sensitive());
    assert!(dock.minimize_animation.is_sensitive());
    assert_eq!(writes.get(), 0, "context changes must not save preferences");
    assert_eq!(dock.selected(), saved);
    dock.set_context(active);
    dock.extend_to_edges.set_active(false);
    dock.show_running.set_active(true);
    assert!(dock.edge_margin.is_sensitive());
    assert!(dock.running_apps_position.is_sensitive());
    assert_eq!(dock.edge_margin.value_as_int(), 17);

    let menu = MenuPage::new();
    let mut saved = menu.selected();
    saved.panel_menu_position = "right".into();
    saved.floating_panel = true;
    saved.panel_margin = 19;
    saved.show_system_menu = false;
    menu.show(&saved);
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    menu.connect_changed(move |_| count.set(count.get() + 1));
    menu.set_context(SettingsContext {
        fixed_menu_position: true,
        ..active
    });
    assert_eq!(menu.panel_menu_position.selected(), 0);
    assert!(!menu.panel_menu_position.is_sensitive());
    assert!(!menu.show_system_about.is_sensitive());
    assert_eq!(
        menu.selected(),
        saved,
        "displaying fixed left must preserve saved right"
    );
    assert_eq!(writes.get(), 0);
    menu.show(&saved);
    assert_eq!(
        menu.selected(),
        saved,
        "refresh must preserve fixed-profile preferences"
    );
    menu.set_context(active);
    assert_eq!(menu.panel_menu_position.selected(), 2);
    assert!(menu.panel_menu_position.is_sensitive());
    assert!(menu.panel_margin.is_sensitive());
    menu.set_context(SettingsContext {
        extended_dock: true,
        ..active
    });
    assert!(!menu.panel_margin.is_sensitive());
    assert_eq!(menu.panel_margin.value_as_int(), 19);
    menu.set_context(active);
    menu.floating_panel.set_active(false);
    assert!(!menu.panel_margin.is_sensitive());
    assert_eq!(menu.panel_margin.value_as_int(), 19);
    menu.set_context(SettingsContext {
        fixed_panel: true,
        ..active
    });
    assert!(!menu.panel_height.is_sensitive());
    assert!(!menu.floating_panel.is_sensitive());
    menu.set_context(SettingsContext::default());
    assert!(!menu.show_clock.is_sensitive());
    assert!(!menu.show_applications_menu.is_sensitive());
    assert!(!menu.show_search_menu.is_sensitive());

    let lock = ScreensaverPage::new();
    lock.lock_delay.set_value(35.0);
    lock.lock_enabled.set_active(false);
    assert!(!lock.lock_delay.is_sensitive());
    lock.lock_enabled.set_active(true);
    assert!(lock.lock_delay.is_sensitive());
    assert_eq!(lock.selected().lock_delay_secs, 35);
}
