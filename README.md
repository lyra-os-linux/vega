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

Licensed under GPL-3.0. Source code at
[github.com/britors/Vega](https://github.com/britors/Vega).

## Features

- dashboard with system-health information and shortcuts;
- Zypper packages, Flatpak applications, updates, and repositories;
- optional snapshots with Snapper or Timeshift, and backups with Restic;
- hardware inventory, drivers, kernel, and bootloader;
- storage, date, time, and locale;
- network, Wi-Fi, Bluetooth, firewall, VPN, proxy, and IPv4;
- users, services, logs, and live process monitoring;
- wallpaper, screen-lock preferences, and a multi-provider AI assistant.

Features backed by optional programs are shown as unavailable when their
dependency is missing without preventing the other pages from working.

## Architecture

| Component | Technology | Role |
| --- | --- | --- |
| `vega-gtk` | Rust, GTK4, and libadwaita | Unprivileged graphical interface |
| `vega-cli` | Bash and `dialog` | Terminal interface for local or SSH use |
| `lyra-vega-dbus` | Rust and zbus | Typed D-Bus client shared by the GTK interface |
| `vegad` | Go | Daemon that performs authorized system operations |
| `dbus/` | Introspection XML | Public `org.lyraos.Vega1.*` contract between clients and daemon |

`vegad` uses the system bus and is activated on demand by D-Bus. It releases
the bus name and exits after two minutes without activity. Read-only queries do
not require authentication; system-changing actions are protected by granular
polkit rules. The graphical interface never needs to run as root.

Vega CLI is aimed primarily at headless servers. Its `vega` entrypoint requires
an interactive terminal and runs as the session user; polkit requests
authentication only when a privileged action is performed.

## Installing on openSUSE

Vega supports openSUSE only. On openSUSE Leap 16.0, the recommended installation
method uses the
[`home:rodrigosbrito:vega`](https://build.opensuse.org/project/show/home:rodrigosbrito:vega)
repository on the openSUSE Build Service:

### Automatic installation

```sh
curl -fsSL https://raw.githubusercontent.com/britors/Vega/main/scripts/install-obs.sh | sudo bash
```

This installs `vega-gtk`, `vegad`, and `vega-cli`, and leaves the repository
configured for future updates through `zypper`.

### Add the OBS repository and install with Zypper

Add the Vega repository:

```sh
sudo zypper addrepo --refresh \
  https://download.opensuse.org/repositories/home:/rodrigosbrito:/vega/openSUSE_Leap_16.0/ \
  vega-obs
```

Refresh its metadata and import the OBS signing key:

```sh
sudo zypper --gpg-auto-import-keys refresh vega-obs
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
curl -fsSL https://raw.githubusercontent.com/britors/Vega/main/scripts/install-obs.sh \
  | sudo env VEGA_CLI_ONLY=1 bash
```

Or, if the repository is already configured:

```sh
sudo zypper install vegad vega-cli
```

After installation, open the graphical interface from the application menu or
run `vega-gtk`. Run `vega` to start the terminal interface.

### Release RPMs

Alternatively, `scripts/install.sh` downloads RPMs from the latest GitHub
release without configuring the OBS repository. A specific tag can be selected
with `VEGA_VERSION=vX.Y.Z`; these standalone RPMs are still installed as
unsigned packages.

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
- Go;
- openSUSE with systemd, D-Bus, and polkit for integration testing.

Validate the Rust interface and client from the repository root:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Validate the daemon:

```sh
cd vegad
GOCACHE=/tmp/vega-gocache go test ./...
```

Run the graphical interface during development:

```sh
cargo run --manifest-path vega-gtk/Cargo.toml
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines,
[vega-gtk/README.md](vega-gtk/README.md) for interface details, and
[dbus/README.md](dbus/README.md) for the D-Bus contract.

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
