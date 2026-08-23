#!/usr/bin/env bash
# Validação estática reproduzível dos contratos e artefatos de pacote.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

for command in desktop-file-validate xmllint rpmspec systemd-analyze; do
  command -v "$command" >/dev/null || {
    echo "dependência ausente: $command" >&2
    exit 1
  }
done

xmllint --noout dbus/*.xml packaging/vegad/org.lyraos.Vega1.conf \
  packaging/vegad/org.lyraos.vega.policy
desktop-file-validate packaging/vega/vega-update-notifier.desktop
desktop-file-validate packaging/vega/org.lyraos.Vega.UpdateNotifier.desktop
desktop-file-validate packaging/vega/vega.desktop
xmllint --noout packaging/vega/icons/lyra-updates-symbolic.svg

# O GNOME so entrega GNotification quando existe um .desktop com o mesmo nome do
# application_id; os dois precisam continuar casados.
notifier_id="$(rg -o 'APPLICATION_ID: &str = "([^"]+)"' -r '$1' \
  vega-gtk/src/bin/vega-update-notifier.rs)"
test -f "packaging/vega/${notifier_id}.desktop" || {
  echo "falta packaging/vega/${notifier_id}.desktop para o app-id do notifier" >&2
  exit 1
}
for spec in packaging/opensuse/vega.spec packaging/obs/vega-gtk.spec; do
  rg -q "applications/${notifier_id}.desktop" "$spec"
done
rg -q "applications/${notifier_id}.desktop" packaging/opensuse/install.sh

rpmspec -P packaging/opensuse/vegad.spec >/dev/null
rpmspec -P packaging/obs/vegad.spec >/dev/null
rpmspec -P packaging/opensuse/vega.spec >/dev/null
rpmspec -P packaging/obs/vega-gtk.spec >/dev/null

# O Sobre deve receber a mesma versão declarada pelo RPM em qualquer build.
for spec in packaging/opensuse/vega.spec packaging/obs/vega-gtk.spec; do
  rg -q 'VEGA_VERSION=%\{version\} cargo build' "$spec"
done
rg -q 'VEGA_VERSION="\$VERSION" cargo build' packaging/opensuse/install.sh
rg -q 'option_env!\("VEGA_VERSION"\)' vega-gtk/src/model.rs
rg -q '\.version\(crate::model::APPLICATION_VERSION\)' vega-gtk/src/ui/shell.rs

verify_output="$(systemd-analyze verify \
  packaging/vegad/vegad.service \
  packaging/vegad/vegad-update-check.service \
  packaging/vegad/vegad-update-check.timer \
  packaging/vegad/vegad-update-check-retry.timer \
  packaging/vegad/vegad-log-export.service \
  packaging/vegad/vegad-log-export.timer \
  packaging/vega-web/vega-web.service 2>&1 || true)"
unexpected="$(printf '%s\n' "$verify_output" | \
  rg '^(vegad|vegad-update-check|vegad-log-export|vega-web)\.' | \
  rg -v 'Command .+ is not executable: No such file or directory' || true)"
if [ -n "$unexpected" ]; then
  printf '%s\n' "$unexpected" >&2
  exit 1
fi
systemd-analyze security --offline=yes --no-pager \
  packaging/vegad/vegad.service packaging/vega-web/vega-web.service >/dev/null

rg -qx 'OnBootSec=1h' packaging/vegad/vegad-update-check.timer
rg -qx 'OnUnitActiveSec=24h' packaging/vegad/vegad-update-check.timer
rg -qx 'RandomizedDelaySec=30min' packaging/vegad/vegad-update-check.timer
rg -qx 'OnActiveSec=2h' packaging/vegad/vegad-update-check-retry.timer
rg -qx 'OnFailure=vegad-update-check-retry.timer' \
  packaging/vegad/vegad-update-check.service
for spec in packaging/opensuse/vegad.spec packaging/obs/vegad.spec; do
  rg -q 'vegad-update-check-retry.timer' "$spec"
done

for action in \
  org.lyraos.vega.logs.read-admin \
  org.lyraos.vega.backup.restore \
  org.lyraos.vega.software.update; do
  rg -q "<action id=\"${action}\">" packaging/vegad/org.lyraos.vega.policy
done

echo "Empacotamento, unidades systemd, XML D-Bus e políticas: OK"
