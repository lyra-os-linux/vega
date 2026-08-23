#!/usr/bin/env bash
# Validação estática reproduzível dos artefatos de pacote do vega-gtk.
#
# Desde a quebra do monorepo, vegad/vega-cli/vega-web/lyra-vega-dbus têm
# repositórios próprios (cada um com sua própria validação de
# empacotamento); este script cobre só o que ainda mora aqui.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

for command in desktop-file-validate xmllint rpmspec; do
  command -v "$command" >/dev/null || {
    echo "dependência ausente: $command" >&2
    exit 1
  }
done

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

rpmspec -P packaging/opensuse/vega.spec >/dev/null
rpmspec -P packaging/obs/vega-gtk.spec >/dev/null

# O Sobre deve receber a mesma versão declarada pelo RPM em qualquer build.
for spec in packaging/opensuse/vega.spec packaging/obs/vega-gtk.spec; do
  rg -q 'VEGA_VERSION=%\{version\} cargo build' "$spec"
done
rg -q 'VEGA_VERSION="\$VERSION" cargo build' packaging/opensuse/install.sh
rg -q 'option_env!\("VEGA_VERSION"\)' vega-gtk/src/model.rs
rg -q '\.version\(crate::model::APPLICATION_VERSION\)' vega-gtk/src/ui/shell.rs

echo "Empacotamento do vega-gtk: OK"
