#!/usr/bin/env bash
# Manual install script for openSUSE Leap — mirrors the layout
# packaging/opensuse/vega.spec installs via RPM, done by hand for a
# from-source install without going through rpmbuild/OBS.
#
# Since the monorepo split, this only builds/installs vega-gtk. vegad
# (daemon) has its own repository and install script:
# https://github.com/lyra-os-linux/vegad
#
# Usage: sudo packaging/opensuse/install.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo $0)" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION="${VEGA_VERSION:-$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" | cut -d'"' -f2)}"
echo "==> Instalando vega-gtk $VERSION (openSUSE Leap) a partir de $REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Erro: 'cargo' é necessário para compilar e não foi encontrado no PATH." >&2
  exit 1
fi

echo "==> Verificando dependências opcionais de runtime"
for tool in flatpak restic snapper firewall-cmd fwupdmgr nmcli bluetoothctl; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "  aviso: '$tool' não encontrado — o recurso correspondente do Vega ficará indisponível até instalar (zypper install ...)"
  fi
done

echo "==> Compilando vega-gtk"
(
  cd "$REPO_ROOT/vega-gtk"
  VEGA_VERSION="$VERSION" cargo build --release --locked
)

echo "==> Instalando vega-gtk (app)"
install -Dm755 "$REPO_ROOT/target/release/vega-gtk" /usr/bin/vega-gtk
install -Dm755 "$REPO_ROOT/target/release/vega-update-notifier" /usr/bin/vega-update-notifier
ln -sfn vega-gtk /usr/bin/lyra-vega-gtk

install -Dm644 "$REPO_ROOT/packaging/vega/vega.desktop" /usr/share/applications/vega.desktop
# O GNOME resolve o app-id da notificação em /usr/share/applications; sem este
# arquivo o GtkNotificationDaemon descarta a notificação em silêncio.
install -Dm644 "$REPO_ROOT/packaging/vega/org.lyraos.Vega.UpdateNotifier.desktop" /usr/share/applications/org.lyraos.Vega.UpdateNotifier.desktop
install -Dm644 "$REPO_ROOT/packaging/vega/vega-update-notifier.desktop" /etc/xdg/autostart/vega-update-notifier.desktop
install -Dm644 "$REPO_ROOT/packaging/vega/vega.svg" /usr/share/icons/hicolor/scalable/apps/vega.svg
install -Dm644 "$REPO_ROOT/packaging/vega/icons/lyra-updates-symbolic.svg" /usr/share/icons/hicolor/symbolic/apps/lyra-updates-symbolic.svg

cat <<EOF

Instalação concluída.
- App: /usr/bin/vega-gtk (ou pelo atalho "Vega" no menu)

vega-gtk precisa do daemon vegad rodando (ativado sob demanda via D-Bus,
org.lyraos.Vega1) para funcionar. Instale-o a partir do repositório
próprio: https://github.com/lyra-os-linux/vegad
EOF
