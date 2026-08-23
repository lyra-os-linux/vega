#!/usr/bin/env bash
# Reverses packaging/opensuse/install.sh (vega-gtk only — see that script
# for why vegad moved to its own repo/uninstaller).
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo $0)" >&2
  exit 1
fi

echo "==> Removendo binários e app"
rm -f /usr/bin/vega-gtk
rm -f /usr/bin/lyra-vega-gtk
rm -f /usr/bin/vega-update-notifier
rm -f /usr/share/applications/vega.desktop
rm -f /usr/share/applications/org.lyraos.Vega.UpdateNotifier.desktop
rm -f /etc/xdg/autostart/vega-update-notifier.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/vega.svg
rm -f /usr/share/icons/hicolor/symbolic/apps/lyra-updates-symbolic.svg

echo "vega-gtk removido."
