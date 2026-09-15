# Vega — Control Center

*[Leia em português](README.pt-br.md)*

Vega is a native control center built exclusively for openSUSE. It brings
software, hardware, kernel, network, backup, user, and service administration
into a single interface integrated with GNOME. It complements GNOME Settings
with administration tasks that would otherwise require separate tools such as
`zypper`, `nmcli`, `systemctl`, and configuration-file editors.

The project provides a graphical interface built with Rust and
GTK4/libadwaita, plus a terminal interface built with Bash and `dialog`. Both
use the same privileged daemon and D-Bus contract.

Licensed under GPL-3.0. This repository hosts `vega-gtk` and the
product-wide docs/scripts; the other components each have their own
repository — see [Architecture](#architecture) below.

## Features

- dashboard with system-health information and shortcuts;
- Zypper packages, Flatpak applications, updates, and repositories;
- optional snapshots with Snapper or Timeshift, and backups with Restic;
- hardware inventory, kernel, and bootloader;
- storage, date, time, and locale;
- network, Wi-Fi, Bluetooth, firewall, VPN, proxy, and IPv4;
- users, services, logs, and live process monitoring;
- wallpaper, screen-lock preferences, and a multi-provider AI assistant.

Driver installation and switching, including NVIDIA and optional hardware firmware,
are no longer offered by Vega. Hardware inventory and GPU monitoring remain available.

Features backed by optional programs are shown as unavailable when their
dependency is missing without preventing the other pages from working.

Personalization offers Lyra, Ubuntu, GNOME Vanilla, Lyra Classic, Lyra Central,
and Lyra Floating desktop profiles.
Vega requires Sheliak 2.0.0 or newer. Ubuntu extends the left dock, aligns apps
at the top, shows GNOME's workspace button, and hides the topbar menus and search.
Workspace button visibility is saved per profile; older Ubuntu snapshots adopt
the visible default once without resetting the other profiles or custom layouts.
Returning to Lyra restores
the previous dock and menu preferences, including through GNOME Vanilla.
Extending the dock manually in Lyra keeps its menus and search visible.
Compact and extended docks keep separate alignment preferences.

Lyra Classic and Lyra Central use a fixed
bottom taskbar with the native clock, calendar, notifications and system
controls. Lyra Classic aligns applications left and opens an application list
with pinned tiles; Lyra Central centers applications and opens a search field
with a pinned grid. The L button and Super key open the corresponding menu.
Each profile's previous preferences are restored when switching back, including
the Lyra snapshot created by earlier Ubuntu-profile versions. All components
continue to be packaged as RPMs.

## Architecture

Vega is split across several repositories under
[lyra-os-linux](https://github.com/lyra-os-linux). This repository
(`vega`) hosts `vega-gtk` plus the docs/scripts that span the whole
product; each other component has its own repository and release cycle:

| Component | Technology | Role | Repository |
| --- | --- | --- | --- |
| `vega-gtk` | Rust, GTK4, and libadwaita | Unprivileged graphical interface | this repo |
| `vega-cli` | Bash and `dialog` | Terminal interface for local or SSH use | [lyra-os-linux/vega-cli](https://github.com/lyra-os-linux/vega-cli) |
| `vega-web` | Rust, axum | HTTPS panel for LAN-only administration | [lyra-os-linux/vega-web](https://github.com/lyra-os-linux/vega-web) |
| `vegad` | Go | Daemon that performs authorized system operations | [lyra-os-linux/vegad](https://github.com/lyra-os-linux/vegad) |
| `lyra-vega-dbus` + `dbus/` | Rust/zbus + introspection XML | Typed D-Bus client and the public `org.lyraos.Vega1.*` contract, shared by the Rust frontends | [lyra-os-linux/lyra-vega-dbus](https://github.com/lyra-os-linux/lyra-vega-dbus) |

Building the full product locally means cloning the repos above as
siblings (e.g. under the same parent directory) — see
[CONTRIBUTING.md](CONTRIBUTING.md).

`vegad` uses the system bus and is activated on demand by D-Bus. It releases
the bus name and exits after two minutes without activity. Read-only queries do
not require authentication; system-changing actions are protected by granular
polkit rules. The graphical interface never needs to run as root.

Vega CLI is aimed primarily at headless servers. Its `vega` entrypoint requires
an interactive terminal and runs as the session user; polkit requests
authentication only when a privileged action is performed.

## Installing on openSUSE

Vega GTK targets GNOME on openSUSE. On the Lyra base, openSUSE Leap 16.1, the recommended installation
method uses the
[`home:rodrigosbrito:vega`](https://build.opensuse.org/project/show/home:rodrigosbrito:vega)
repository on the openSUSE Build Service:

### Add the OBS repository and install with Zypper

Add both repositories for the graphical interface. Sheliak supplies the Lyra
GNOME extensions and schemas used by Vega:

```sh
sudo zypper addrepo --refresh \
  https://download.opensuse.org/repositories/home:/rodrigosbrito:/vega/openSUSE_Leap_16.1/ \
  vega-obs
sudo zypper addrepo --refresh \
  https://download.opensuse.org/repositories/home:/rodrigosbrito:/lyra/openSUSE_Leap_16.1/ \
  lyra-obs
```

Refresh its metadata and import the OBS signing key:

```sh
sudo zypper --gpg-auto-import-keys refresh vega-obs lyra-obs
```

Install the graphical interface, daemon, and terminal interface:

```sh
sudo zypper install vega-gtk vegad vega-cli
```

`vegad` is activated automatically over D-Bus when an interface needs it; it
does not need to be started manually.

To update Vega later:

```sh
sudo zypper refresh vega-obs
sudo zypper update
```

### Headless installation

To install only the daemon and terminal interface on a headless machine:

```sh
sudo env VEGA_CLI_ONLY=1 bash scripts/install-obs.sh
```

Run this from a reviewed checkout. The helper detects the Leap version,
including the upstream `/usr/lib/os-release` on Lyra. Headless installation
configures only the Vega repository. An existing alias pointing to another
base version is rejected before package installation.

Or, if the repository is already configured:

```sh
sudo zypper install vegad vega-cli
```

After installation, open the graphical interface from the application menu or
run `vega-gtk`. Run `vega` to start the terminal interface.

### Release RPMs

Alternatively, `scripts/install.sh` downloads RPMs from the latest GitHub
release of each component's repository (`vega`, `vegad`, `vega-cli`)
using the Lyra OBS repository for GTK runtime dependencies, including Sheliak.
Run it from a reviewed checkout. A specific tag can be selected with
`VEGA_VERSION=vX.Y.Z` (used against all three repos, so it only works if
their releases share that tag); these standalone RPMs are still installed
as unsigned packages.

The GTK RPM requires the three bundled translation catalogs, `glibc-locale-base`,
GTK/libadwaita runtime libraries, vegad, and the GNOME applications opened by
the personalization cards. Development packages are not required to run Vega.
Publish compatible Vega, vegad and Sheliak builds together: Vega's suite
dependency cannot be satisfied by the older Sheliak 1.x packages.

## Interface language

Open **Main menu → Settings → Vega language** to select Portuguese (Brazil),
English, Spanish, or **Follow system language**. Reopen Vega to apply the
choice. It is saved in the user's preferences and does not change the GNOME
system language or require administrator authentication.

Automatic selection honors `LANGUAGE` (including preference lists), followed
by `LC_ALL`, `LC_MESSAGES` and `LANG`; portable C/POSIX entries are skipped.
Other Portuguese/Spanish/English regions use the corresponding bundled
translation. Unsupported languages fall back to English. Installed binaries
read their RPM catalogs from the system, independently of the source checkout.

## Uninstalling

```sh
sudo bash scripts/uninstall.sh
```

The script removes any installed `vega-gtk`, `vegad`, and `vega-cli` packages.
Set `VEGA_PURGE=1` to also delete backup configuration under `/etc/vega` and
exported logs under `/var/log/vega`.

Per-user assistant preferences in
`~/.local/share/vega-gtk/ai-settings.json` are preserved.

## Development

Prerequisites:

- Rust 1.92 or newer, GTK4, and libadwaita;
- openSUSE with systemd, D-Bus, and polkit for integration testing
  (needs `vegad` installed/running — see its own repository).

Validate the Rust interface and client from the repository root:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The daemon (`vegad`) has its own repository, tests, and validation —
see [lyra-os-linux/vegad](https://github.com/lyra-os-linux/vegad).

Run the graphical interface during development:

```sh
cargo run --manifest-path vega-gtk/Cargo.toml
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines,
[vega-gtk/README.md](vega-gtk/README.md) for interface details, and
[lyra-vega-dbus](https://github.com/lyra-os-linux/lyra-vega-dbus) for the
D-Bus contract.

## Tested openSUSE versions

- openSUSE Leap 16
- openSUSE Tumbleweed

## Known limitations

- Other Linux distributions are not supported.
- Zypper and Flatpak progress is reported per step rather than per byte
  transferred.
- Snapper and Timeshift are optional. Advanced diff and retention features
  remain Snapper-specific.
- Wi-Fi, Bluetooth, screen settings, and the AI assistant belong to a graphical
  session and are not included in Vega CLI.

## Assistant privacy

The assistant is optional. Keys are stored in the session Secret Service, and
system-changing actions are presented as proposals before anything is executed.
See [docs/ai-privacidade.md](docs/ai-privacidade.md) for details.

### Desktop profile command

Vega GTK 5.1.33 exposes a local command for setup applications such as Lyra
Welcome, without opening a GTK window or contacting vegad:

```sh
vega-gtk --desktop-profile get
vega-gtk --desktop-profile set windows10
```

IDs: `lyra`, `vanilla`, `ubuntu`, `windows10`, `windows11`, `macos`.
The current suite requires Vega GTK 5.1.35 and Sheliak 2.0.0. On success stdout
contains exactly the stored profile ID. Exit 1 means unavailable settings or
an application/readback error; exit 2 means invalid arguments. This command
uses the same save/restore implementation as Vega's profile cards, flushes
settings before exiting and confirms the stored profile. It requires a current
Sheliak installation with per-profile preference storage. A disabled extension
backend or missing schema is an error, not an inferred Lyra/Vanilla selection.
Readback confirms settings, not completion of Shell rendering. Concurrent
external changes after that read are outside this command's confirmation.

`tests/profile-command.py` checks the built command against real GSettings in a
private persistent keyfile backend. Pass `--suite-helper` with the current
Sheliak helper when testing the six-extension suite. The legacy test without
that option requires an environment without system-wide suite extensions.
`tests/native-ubuntu-workspaces/extension.js` runs through Sheliak's
`tests/native-pins/run.py --probe` with `LYRA_NATIVE_VEGA_BINARY` pointing to the
built Vega binary and `LYRA_NATIVE_SUITE_HELPER` to Sheliak's
`packaging/lyra-shell-suite.py`. It validates profile switches, panel toggling
and clicks opening and closing the GNOME workspace overview in a private
compositor. `tests/native-profiles/extension.js` is the older monolithic
extension probe, using `VEGA_PROFILE_TEST_BINARY`.

### Desktop icons and Lyra Floating

The Lyra Floating preset uses a centered floating bottom dock with icon magnification,
a flush top bar and the Lyra application logo at the left. Each profile restores
its own alignment, animation, margin and menu position, including snapshots saved
before these fields existed.

In Personalization → Desktop profile, **Active desktop** enables or disables
Lyra Desktop Icons (`desktop-icons@lyraos.com.br`), included in the Sheliak RPM.
Disabling hides desktop icons without moving or deleting files. This preference
is independent of all six profiles. Separate switches control Lyra Dock, Panel,
Menus, Search and Animations; selections are saved per layout. GNOME Vanilla
keeps the five shell components off while preserving the desktop icon choice.
The UI respects global extension disable and reports persistence failures.

The unprivileged `/usr/libexec/lyra/shell-suite` helper migrates old UUIDs and
journals a profile change before Vega writes its layout. Commit finalizes the
component selection; abort or recovery after process interruption restores the
owned preferences. A concurrent profile transaction is rejected. Third-party
extensions and Desktop files are preserved. Welcome delegates to the same Vega
command; vegad needs no new privileged operations.

Native regression tests use Sheliak's `tests/native-pins/run.py` with
`--probe tests/native-suite/extension.js`. Point `LYRA_NATIVE_SUITE_HELPER` at
its helper, `LYRA_NATIVE_VEGA_BINARY` at the built Vega and
`VEGA_DESKTOP_TEST_BINARY` at the Cargo test executable. The runner uses a
private HOME and compositor, including real GTK switch callbacks. The legacy
command fixture remains to cover compatibility with previous schema versions.
