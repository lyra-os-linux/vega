#!/usr/bin/env bash
# Manual install script for openSUSE Leap — mirrors the layout the
# packaging/opensuse/*.spec files install via RPM, done by hand for a
# from-source install without going through rpmbuild/OBS.
#
# Usage: sudo packaging/opensuse/install.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo $0)" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION="${VEGA_VERSION:-$(grep -m1 '^var Version' "$REPO_ROOT/vegad/internal/version/version.go" | cut -d'"' -f2)}"
echo "==> Instalando Vega/vegad $VERSION (openSUSE Leap) a partir de $REPO_ROOT"

for tool in go cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Erro: '$tool' é necessário para compilar e não foi encontrado no PATH." >&2
    exit 1
  fi
done

echo "==> Verificando dependências opcionais de runtime"
for tool in flatpak restic snapper firewall-cmd fwupdmgr nmcli bluetoothctl; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "  aviso: '$tool' não encontrado — o recurso correspondente do Vega ficará indisponível até instalar (zypper install ...)"
  fi
done

echo "==> Compilando vegad"
(
  cd "$REPO_ROOT/vegad"
  # This script runs as root (see the check above) against a checkout that
  # normally belongs to the user. Go stamps VCS metadata by shelling out to
  # git, which refuses a repository it does not own ("dubious ownership") and
  # fails the build — so the documented 'sudo install.sh' never got past this
  # step. The stamp is not used anywhere; VERSION above is what the About
  # screen reads.
  go build -trimpath -buildvcs=false \
    -ldflags "-X github.com/lyraos/vegad/internal/version.Version=${VERSION}" \
    -o vegad ./cmd/vegad
)

echo "==> Compilando vega-gtk"
(
  cd "$REPO_ROOT/vega-gtk"
  VEGA_VERSION="$VERSION" cargo build --release --locked
)

echo "==> Instalando vegad e integração systemd/D-Bus/polkit"
install -Dm755 "$REPO_ROOT/vegad/vegad" /usr/lib/vega/vegad
install -Dm644 "$REPO_ROOT/packaging/vegad/vegad.service" /usr/lib/systemd/system/vegad.service
install -Dm644 "$REPO_ROOT/packaging/vegad/vegad-update-check.service" /usr/lib/systemd/system/vegad-update-check.service
install -Dm644 "$REPO_ROOT/packaging/vegad/vegad-update-check.timer" /usr/lib/systemd/system/vegad-update-check.timer
install -Dm644 "$REPO_ROOT/packaging/vegad/vegad-update-check-retry.timer" /usr/lib/systemd/system/vegad-update-check-retry.timer
install -Dm644 "$REPO_ROOT/packaging/vegad/vegad.conf" /etc/vega/vegad.conf
install -Dm644 "$REPO_ROOT/packaging/vegad/profiles/desktop.conf" /usr/share/vega/profiles/desktop.conf
install -Dm644 "$REPO_ROOT/packaging/vegad/profiles/server.conf" /usr/share/vega/profiles/server.conf
install -Dm644 "$REPO_ROOT/packaging/vegad/org.lyraos.Vega1.conf" /usr/share/dbus-1/system.d/org.lyraos.Vega1.conf
install -Dm644 "$REPO_ROOT/packaging/vegad/org.lyraos.Vega1.service" /usr/share/dbus-1/system-services/org.lyraos.Vega1.service
install -Dm644 "$REPO_ROOT/packaging/vegad/org.lyraos.vega.policy" /usr/share/polkit-1/actions/org.lyraos.vega.policy

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

echo "==> Recarregando systemd/D-Bus e habilitando o timer de atualização"
systemctl daemon-reload
systemctl reload dbus.service 2>/dev/null || true
systemctl enable --now vegad-update-check.timer

cat <<EOF

Instalação concluída.
- Daemon: /usr/lib/vega/vegad (ativado sob demanda via D-Bus, org.lyraos.Vega1)
- App: /usr/bin/vega-gtk (ou pelo atalho "Vega" no menu)

Aviso: o backend Zypper/hardware NVIDIA do vegad para openSUSE (vegad/internal/distro/zypper.go,
hardware_opensuse.go) ainda não foi validado ponta a ponta num Leap real — teste os módulos de
Software/Kernel/Hardware com cautela antes de confiar neles em produção.
EOF
