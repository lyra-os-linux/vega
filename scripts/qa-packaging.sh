#!/usr/bin/env bash
# Validação estática reproduzível dos contratos e artefatos de pacote.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

for command in xmllint rpmspec systemd-analyze; do
  command -v "$command" >/dev/null || {
    echo "dependência ausente: $command" >&2
    exit 1
  }
done

xmllint --noout dbus/*.xml packaging/vegad/org.lyraos.Vega1.conf \
  packaging/vegad/org.lyraos.vega.policy

rpmspec -P packaging/opensuse/vegad.spec >/dev/null
rpmspec -P packaging/obs/vegad.spec >/dev/null
rpmspec -P packaging/opensuse/vega.spec >/dev/null
rpmspec -P packaging/obs/vega-gtk.spec >/dev/null

verify_output="$(systemd-analyze verify \
  packaging/vegad/vegad.service \
  packaging/vegad/vegad-update-check.service \
  packaging/vegad/vegad-update-check.timer \
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

for action in \
  org.lyraos.vega.logs.read-admin \
  org.lyraos.vega.backup.restore \
  org.lyraos.vega.software.update; do
  rg -q "<action id=\"${action}\">" packaging/vegad/org.lyraos.vega.policy
done

echo "Empacotamento, unidades systemd, XML D-Bus e políticas: OK"
