# vega-gtk/src/backend

## Overview

The backend is the Rust side of the Tauri IPC surface: the
`#[tauri::command]` functions that the web UI calls through `call(...)` in
`ui/js/api.js`. Most commands are thin async wrappers around the typed
`lyra_vega_dbus` clients. Those clients talk to the privileged daemon `vegad`
over the system bus (`org.lyraos.Vega1`), and `vegad` enforces polkit
authorization. A few domains run unprivileged work directly in the user
session (Flatpak, Wine, AppImage, npm, fwupd, user cleanup, GPU benchmark).
They stream progress through the same Tauri event as vegad transactions.

## Interface

- `mod.rs`: shared plumbing.
  - `Backend`, the managed Tauri state, holds
    `dbus: Mutex<Option<VegaDbus>>`. `Backend::dbus()` connects lazily through
    `VegaDbus::connect()` and caches only a successful connection.
  - `State<'a>` and `CmdResult<T> = Result<T, String>`.
  - `err()` maps any `Display` to a string. Client errors are already
    gettext-translated.
  - The `dto!` macro builds serde DTOs that mirror the D-Bus types, and
    `list()` converts whole lists.
  - `mod.rs` declares one `pub mod` for each file below.
- vegad-backed commands. The bullets list each file's commands; each file
  wraps the `lyra_vega_dbus` client named in parentheses.
  - `software.rs` (`software()`):
    - `software_search`, `software_details`, `software_installed`,
      `software_updates`, `software_update_status`, `software_request_check`,
      `software_repos`, `software_set_repo_enabled`.
    - `software_run(Action)`. The `kind` tag is one of `install`, `remove`,
      `update-package`, `update-all`, `update-all-native`, `clear-cache`,
      `refresh-repos`, `add-repo`, `trust-repo-key`, `update-firmware` or
      `approve-preparation-key`.
    - `pub fn watch_updates`.
    - `pub enum TransactionEvent`. Its `type` tag is one of `progress`,
      `package`, `console`, `key-pending` or `finished`.
  - `network.rs` (`network()` and `firewall()`):
    - Network: `network_interfaces`, `network_wifi`, `network_proxy`,
      `network_set_proxy`, `network_connect_wifi`, `network_disconnect`,
      `network_set_static_ipv4`, `network_import_vpn`.
    - Firewall: `firewall_status`, `firewall_services`, `firewall_ports`,
      `firewall_set_service`, `firewall_add_port`, `firewall_remove_port`,
      `firewall_listening`.
  - `backup.rs` (`backup()`):
    - `backup_configs`, `backup_create_config`, `backup_delete_config`,
      `backup_snapshots`, `backup_snapshot_paths`.
    - `backup_run`, which takes `run-now` or `restore-items`.
    - `pub fn watch_alerts`.
  - `snapshots.rs`: `snapshots_available`, `snapshots_list`,
    `snapshots_create`, `snapshots_diff`, `snapshots_rollback`,
    `snapshots_delete`, `snapshots_set_retention`, `snapshots_clear`.
  - `users.rs`: `users_list`, `users_save`, `users_set_admin`, `users_remove`,
    `users_open_parental_controls`.
  - `bluetooth.rs`: `bluetooth_status`, `bluetooth_devices`,
    `bluetooth_set_powered`, `bluetooth_set_scanning`,
    `bluetooth_device_action`, `bluetooth_send_file`,
    `bluetooth_start_receiver`.
  - `diagnostics.rs`:
    - `diagnostics_tests`, `diagnostics_current`, `diagnostics_start`.
    - `diagnostics_repair`, `diagnostics_repair_findings`,
      `diagnostics_remedies`, `diagnostics_mark_logs_seen`.
    - `diagnostics_last_periodic`, `diagnostics_get_periodic`,
      `diagnostics_set_periodic`.
    - `diagnostics_follow`, `diagnostics_cancel`, `diagnostics_report`,
      `diagnostics_export`.
    - `Event` is tagged by `type`: `progress`, `sample`, `stage`, `finished`
      or `novelties`.
  - `cleanup.rs` (system part): `cleanup_scan`, `cleanup_start`.
  - `kernel.rs` (`KernelClient`): `kernel_installed`, `kernel_available`,
    `kernel_boot_status`, `kernel_boot_entries`, `kernel_apply_boot`.
  - `services.rs` (`ServicesClient`): `services_list`,
    `services_set_enabled`, `services_set_running`, `services_restart`.
  - `monitor.rs` (`MonitorClient`): `monitor_metrics`, `monitor_processes`,
    `monitor_kill`.
  - `system.rs` (`SystemClient` and `MetadataClient`): `system_status`,
    `system_disk_usage`, `system_metadata`.
  - `datetime.rs` (`DateTimeClient`): `datetime_load`, `datetime_apply`.
  - `storage.rs` (`StorageClient`): `storage_list`, `storage_mount`,
    `storage_unmount`.
  - `preparation.rs` (`PreparationClient`): `preparation_status`,
    `preparation_pending_keys`, `preparation_retry`.
  - `logs.rs` (`LogsClient`): `logs_units`, `logs_query`.
  - `hardware.rs` (`HardwareClient`): `hardware_inventory`,
    `hardware_firmware`.
- Session-side commands. These run unprivileged in the user session without
  vegad.
  - `flatpak_session.rs`:
    - `start(app, Operation, id)`, where `Operation` is `Install`, `Update` or
      `Remove`.
    - `valid_app_id`.
    - `pub(crate) follow` and `follow_then`: a generic process-to-event
      streamer that `npm_apps` and `windows_apps` reuse.
  - `windows_apps.rs`:
    - Commands: `windows_ia32`, `windows_installed`, `windows_install`,
      `windows_remove`, `windows_run_file`.
    - Catalogs: `VERBS`, `RECIPES`, `REMOVALS`.
    - `guarded_winetricks`.
  - `appimage_apps.rs`: `appimage_installed`, `appimage_install`,
    `appimage_remove`, and the `APPS` allowlist (`artcraft-launcher`).
  - `npm_apps.rs`: `npm_installed`, `npm_install`, `npm_remove`, and the
    `PACKAGES` allowlist.
  - `fwupd_update.rs`: `start(app)`, called from `software_run`
    `update-firmware`.
  - `cleanup.rs` (user part): `cleanup_user_scan`, `cleanup_user_clean`.
  - `gpu_bench.rs`: `gpu_bench`, `gpu_bench_cancel`, and the parsers
    `parse_vulkan_summary`, `parse_egl_renderers`, `parse_glmark2` and
    `classify_rtd3`.
  - `power.rs`: `system_boot_id`, `system_reboot`.
  - `notify.rs`: `notify`.
  - `commands_present.rs`: `commands_present(names)` (`PATH` lookup).
- `fixtures/` holds text captures used only by `gpu_bench` unit tests. It is
  not a contract.

## Invariants

- Subscribe before start. `software_run`, `backup_run`, `cleanup_start`,
  `diagnostics_start`, `diagnostics_repair` and
  `diagnostics_repair_findings` call the D-Bus `subscribe()` before they
  start the job, so a fast `Finished` is never missed.
- Each follower forwards only events whose transaction or job id matches. It
  stops at the terminal event and turns a stream error into a synthetic
  failed `Finished`.
- `diagnostics.rs` allows at most one follower per job (static `FOLLOWED`
  set). `diagnostics_follow` does nothing for job `0` or a job that is
  already followed.
- Origin routing in `software_run`:
  - `install`, `update-package` and `remove` with `origin == "flathub"` go to
    `flatpak_session::start` and never reach vegad.
  - `update-firmware` goes to `fwupd_update::start`.
  - Everything else goes to vegad.
- Session-side transaction ids use fixed high ranges so they do not collide
  with vegad ids:
  - Flatpak: `0x8000_0000`
  - Wine: `0xC000_0000`
  - npm: `0xD000_0000`
  - AppImage: `0xE000_0000`
  - fwupd: `0xC000_0000`. This is the same base as `windows_apps.rs`, so
    concurrent Wine and firmware tasks can emit colliding ids.
- Allowlists. The UI cannot pass free text to installers.
  - `windows_install` accepts only ids in `VERBS` or `RECIPES`.
  - `windows_remove` accepts only ids in `REMOVALS`.
  - `appimage_apps::start` accepts only ids in `APPS`.
  - `npm_apps::start` accepts only names in `PACKAGES`: `@openai/codex`,
    `@google/gemini-cli`, `@github/copilot`, `opencode-ai`,
    `@mermaid-js/mermaid-cli`.
  - `bluetooth_device_action` accepts only `pair`, `connect` or `disconnect`.
- Input validation:
  - `flatpak_session::valid_app_id` requires at least 3 dot-separated parts
    of `[A-Za-z0-9_-]`, no part starting with a digit, no leading `-`, and at
    most 255 bytes. The id is always passed after `--`.
  - `users.rs:valid_username` requires the first character to be `[a-z_]` and
    the rest to be `[a-z0-9_-]`.
  - Photos are limited to 5 MB (`PHOTO_LIMIT`).
  - `windows_run_file` requires an absolute path to an existing `.exe` or
    `.msi` file.
- Integrity of downloads:
  - Wine recipes (`fetch_verified`) download over HTTPS only, must match
    `recipe.sha256` before the file is renamed from `.part`, and time out
    after 600 s.
  - AppImages must match the release `SHA256SUMS.txt` (`https_only`, 900 s
    timeout).
  - The guarded winetricks copy refuses to run unless `WINETRICKS_AUTO_YES`
    appears exactly once in `/usr/bin/winetricks`.
- User cleanup safety:
  - `cleanup.rs` never follows symlinks (`symlink_metadata`) and refuses a
    symlinked `~/.cache`.
  - Cache entries must be plain names (no `/`, `.`, `..` or `thumbnails`).
  - It refuses entries that a running process holds open (scan of
    `/proc/<pid>/fd` and `cwd`).
- `gpu_bench.rs`:
  - It requires mains power.
  - It reports `unavailable` when `vulkaninfo`, `eglinfo` or
    `glmark2-es2-wayland` is missing.
  - Child processes are killed on cancel or timeout (30 s, or 240 s for
    glmark2).
  - The NVIDIA power check reads only sysfs and `/proc`. It never opens
    `/dev/nvidia*`.
- `software.rs:icon_data_url` inlines only absolute png, svg or jpg files of
  at most 96 KiB (`ICON_LIMIT`).
- `watch_updates` and `watch_alerts` open their own `VegaDbus::connect()`.
  They retry every 5 s forever when the connection or subscription fails.
- Every spawned `Command` goes through `crate::webkit_env::clean_command`,
  which strips the WebKit environment.

## Side Effects

- D-Bus (system bus) calls to vegad through the `lyra_vega_dbus` clients.
  These cover:
  - software, repositories and preparation keys;
  - network, firewall, backup, snapshots and users;
  - bluetooth, diagnostics, cleanup, kernel, services and monitor (including
    `kill_process`);
  - system, datetime, storage, logs and hardware.
- D-Bus (session bus): `notify.rs` calls `org.freedesktop.Notifications`
  `Notify`.
- Tauri events emitted:
  - `software-transaction`, from `software`, `flatpak_session`,
    `windows_apps`, `appimage_apps`, `npm_apps` and `fwupd_update`.
  - `updates-available` and `backup-transaction`.
  - `backup-alert`, `snapshots-clear-progress` and `cleanup-event`.
  - `diagnostics-event` and `diagnostics-gpu`.
- Processes spawned. A user-chosen `.exe` or `.msi` file runs inside the
  Wine prefix (`windows_run_file`).
  - Flatpak: `flatpak install|update|uninstall`, `flatpak list`,
    `flatpak uninstall --unused --user`.
  - Wine: `wine`, `wineboot -i`, `wine reg`, `wine msiexec`, and
    `/bin/sh <guarded winetricks>`.
  - Menu databases: `update-desktop-database` and `update-mime-database`.
  - Node and shell: `npm install|uninstall --global --prefix ~/.local` and
    `fish -c fish_add_path`.
  - Firmware: `fwupdmgr update -y --no-reboot-check`.
  - Users: `malcontent-control --user <name>`.
  - GPU benchmark: `vulkaninfo --summary`, `eglinfo -B`,
    `glmark2-es2-wayland --off-screen`.
  - Power: `systemctl reboot`.
- Network:
  - Wine recipe hosts listed in `RECIPES`, for example `www.rarlab.com`,
    `github.com` and `downloads.sourceforge.net`.
  - `api.github.com`, `raw.githubusercontent.com` and the GitHub release
    assets (AppImage).
  - The npm registry.
- Files written or removed:
  - The Wine prefix `~/.local/share/wineprefixes/lyra-windows`. A broken
    prefix is renamed to `.quebrado-<secs>` and never deleted.
  - `~/.cache/lyra-vega/windows/` and
    `$XDG_RUNTIME_DIR/lyra-vega/winetricks`.
  - `~/Applications/<id>.AppImage`.
  - `lyra-appimage-*` and `lyra-windows-*` `.desktop` files and icons under
    `$XDG_DATA_HOME`.
  - Trash, thumbnails, selected `~/.cache` entries, and unused user Flatpak
    runtimes (`cleanup_user_clean`).
  - A diagnostics report at the path the user picks in a save dialog.
- Reads:
  - `/proc/cmdline`, `/boot/config-*`, `/proc/sys/kernel/random/boot_id`.
  - sysfs PCI and power_supply nodes.
  - `/proc/<pid>/{fd,exe,comm,cwd}`.

## Verification

- Run: `cargo test -p vega-gtk --locked`
