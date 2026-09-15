#!/usr/bin/env bash
# Instalador de conveniência via openSUSE Build Service (OBS): adiciona o
# repositório home:rodrigosbrito:vega e, para GNOME, home:rodrigosbrito:lyra.
# Detecta a versão da base Leap; os pacotes GTK continuam em RPM.
#
# Uso a partir de um checkout revisado:
#   sudo bash scripts/install-obs.sh
#
# VEGA_CLI_ONLY=1 sudo -E bash install-obs.sh   # só vegad + vega-cli, sem
#                                                # a interface GTK (e sem
#                                                # puxar gtk4/libadwaita) —
#                                                # pensado pra servidor
#                                                # headless administrado só
#                                                # por SSH.
set -euo pipefail

VEGA_OBS_PROJECT="home:rodrigosbrito:vega"
VEGA_OBS_ALIAS="vega-obs"
VEGA_CLI_ONLY="${VEGA_CLI_ONLY:-0}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root a partir de um checkout revisado (sudo bash scripts/install-obs.sh)." >&2
  exit 1
fi

if ! command -v zypper >/dev/null 2>&1; then
  echo "Erro: 'zypper' não encontrado — este script só cobre openSUSE Leap (via OBS)." >&2
  exit 1
fi

. "$(dirname "${BASH_SOURCE[0]}")/obs-repositories.sh"
target="$(vega_obs_target)"
vega_obs_configure vega "$target"
if [ "$VEGA_CLI_ONLY" != "1" ]; then
  vega_obs_configure lyra "$target"
fi

if [ "$VEGA_CLI_ONLY" = "1" ]; then
  echo "==> VEGA_CLI_ONLY=1: instalando só vegad + vega-cli"
  zypper --non-interactive install vegad vega-cli
else
  echo "==> Instalando vegad + vega-gtk + vega-cli"
  zypper --non-interactive install vega-gtk vegad vega-cli
fi

if [ "$VEGA_CLI_ONLY" = "1" ]; then
  cat <<EOF

Instalação concluída via OBS ($VEGA_OBS_PROJECT).
- Daemon: vegad, ativado sob demanda via D-Bus (org.lyraos.Vega1)
- Interface: /usr/bin/vega (terminal, dialog)

O repositório '$VEGA_OBS_ALIAS' já fica configurado — 'sudo zypper update'
no futuro também atualiza o Vega, sem precisar rodar este script de novo.
EOF
else
  cat <<EOF

Instalação concluída via OBS ($VEGA_OBS_PROJECT).
- Daemon: vegad, ativado sob demanda via D-Bus (org.lyraos.Vega1)
- Interface gráfica: /usr/bin/vega-gtk
- Interface de terminal: /usr/bin/vega (rode via SSH, sem precisar do ambiente gráfico)

O repositório '$VEGA_OBS_ALIAS' já fica configurado — 'sudo zypper update'
no futuro também atualiza o Vega, sem precisar rodar este script de novo.
EOF
fi
