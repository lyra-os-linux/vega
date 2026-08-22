#!/usr/bin/env bash
# Reverses packaging/opensuse/install.sh.
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo $0)" >&2
  exit 1
fi

echo "==> Desabilitando timer e removendo unidades systemd"
systemctl disable --now vegad-update-check.timer 2>/dev/null || true
systemctl stop vegad-update-check-retry.timer 2>/dev/null || true
rm -f /usr/lib/systemd/system/vegad.service
rm -f /usr/lib/systemd/system/vegad-update-check.service
rm -f /usr/lib/systemd/system/vegad-update-check.timer
rm -f /usr/lib/systemd/system/vegad-update-check-retry.timer

echo "==> Removendo integração D-Bus/polkit"
rm -f /usr/share/dbus-1/system.d/org.lyraos.Vega1.conf
rm -f /usr/share/dbus-1/system-services/org.lyraos.Vega1.service
rm -f /usr/share/polkit-1/actions/org.lyraos.vega.policy

echo "==> Removendo binários e app"
rm -f /usr/lib/vega/vegad
rmdir --ignore-fail-on-non-empty /usr/lib/vega 2>/dev/null || true
rm -f /usr/bin/vega-gtk
rm -f /usr/bin/vega-update-notifier
rm -rf /usr/lib/lyra-vega
rm -f /usr/share/applications/vega.desktop
rm -f /etc/xdg/autostart/vega-update-notifier.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/vega.svg
rm -f /usr/share/icons/hicolor/symbolic/apps/lyra-updates-symbolic.svg
rm -rf /usr/share/gnome-shell/extensions/updates-indicator@lyraos.org

echo "==> Recarregando systemd/D-Bus"
systemctl daemon-reload
systemctl reload dbus.service 2>/dev/null || true

echo "Vega/vegad removidos."
