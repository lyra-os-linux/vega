#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod appearance;
mod assistant;
mod backend;
mod dock;
mod i18n;
mod model;
mod personalization;
mod preferences;
mod profile_command;
mod screensaver;
mod tweaks;
mod virtualization;
mod virtualization_messages;
mod webkit_env;

use std::process::ExitCode;

use backend::*;
use tauri::{Emitter, Manager};

fn main() -> ExitCode {
    // Antes de qualquer thread e de o WebKit iniciar (vega#166).
    webkit_env::apply();
    // O contrato de perfil (usado pelo Lyra Welcome) roda antes de qualquer
    // janela e nunca abre uma.
    if let Some(code) = profile_command::run(std::env::args().skip(1).collect()) {
        return code.into();
    }
    i18n::init(&preferences::load().language);
    let started = std::time::Instant::now();
    // A unicidade fica com o plugin de instância única; o GApplication do
    // GTK não pode registrar o mesmo id (ele reativaria a janela "main" e o
    // Tauri entraria em pânico). O prgname mantém o app_id/WM_CLASS
    // org.lyraos.Vega, que liga a janela ao vega.desktop.
    glib::set_prgname(Some("org.lyraos.Vega"));
    glib::set_application_name("Vega");

    let mut builder = tauri::Builder::default();
    // Só em builds de desenvolvimento: várias instâncias lado a lado para
    // testar páginas em paralelo.
    if !(cfg!(debug_assertions) && std::env::var_os("VEGA_DEV_MULTI_INSTANCE").is_some()) {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Segunda execução: reapresenta a janela e repassa o pedido.
            if let Some(page) = app::requested_page(&argv) {
                let _ = app.emit("open-page", page);
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .manage(Backend::default())
        .manage(app::Startup { started })
        .manage(virtualization::Creation::default())
        .setup(|app| {
            software::watch_updates(app.handle().clone());
            backup::watch_alerts(app.handle().clone());
            personalization::watch_accent(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::app_info,
            app::app_ready,
            app::app_log,
            app::i18n_catalog,
            app::prefs_get,
            app::prefs_set,
            app::open_uri,
            notify::notify,
            system::system_status,
            diagnostics::diagnostics_tests,
            diagnostics::diagnostics_current,
            diagnostics::diagnostics_start,
            diagnostics::diagnostics_follow,
            diagnostics::diagnostics_cancel,
            diagnostics::diagnostics_repair,
            diagnostics::diagnostics_report,
            diagnostics::diagnostics_export,
            gpu_bench::gpu_bench,
            gpu_bench::gpu_bench_cancel,
            system::system_disk_usage,
            system::system_metadata,
            preparation::preparation_status,
            preparation::preparation_pending_keys,
            preparation::preparation_retry,
            software::software_search,
            software::software_details,
            software::software_installed,
            software::software_updates,
            software::software_update_status,
            software::software_request_check,
            software::software_repos,
            software::software_set_repo_enabled,
            software::software_nvidia_status,
            software::software_run,
            backup::backup_configs,
            backup::backup_create_config,
            backup::backup_delete_config,
            backup::backup_snapshots,
            backup::backup_snapshot_paths,
            backup::backup_run,
            snapshots::snapshots_available,
            snapshots::snapshots_list,
            snapshots::snapshots_create,
            snapshots::snapshots_diff,
            snapshots::snapshots_rollback,
            snapshots::snapshots_delete,
            snapshots::snapshots_set_retention,
            snapshots::snapshots_clear,
            hardware::hardware_inventory,
            hardware::hardware_firmware,
            kernel::kernel_installed,
            kernel::kernel_available,
            kernel::kernel_boot_status,
            kernel::kernel_boot_entries,
            kernel::kernel_apply_boot,
            storage::storage_list,
            storage::storage_mount,
            storage::storage_unmount,
            network::network_interfaces,
            network::network_wifi,
            network::network_proxy,
            network::network_set_proxy,
            network::network_connect_wifi,
            network::network_disconnect,
            network::network_set_static_ipv4,
            network::network_import_vpn,
            network::firewall_status,
            network::firewall_services,
            network::firewall_ports,
            network::firewall_set_service,
            network::firewall_add_port,
            network::firewall_remove_port,
            network::firewall_listening,
            bluetooth::bluetooth_status,
            bluetooth::bluetooth_devices,
            bluetooth::bluetooth_set_powered,
            bluetooth::bluetooth_set_scanning,
            bluetooth::bluetooth_device_action,
            bluetooth::bluetooth_send_file,
            bluetooth::bluetooth_start_receiver,
            services::services_list,
            services::services_set_enabled,
            services::services_set_running,
            services::services_restart,
            users::users_list,
            users::users_save,
            users::users_set_admin,
            users::users_remove,
            users::users_open_parental_controls,
            logs::logs_units,
            logs::logs_query,
            monitor::monitor_metrics,
            monitor::monitor_processes,
            monitor::monitor_kill,
            datetime::datetime_load,
            datetime::datetime_apply,
            personalization::personalization_call,
            assistant::commands::assistant_call,
            virtualization::vms_call,
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Vega");
    ExitCode::SUCCESS
}
