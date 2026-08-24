#!/usr/bin/env bash
# Instalador de conveniência via openSUSE Build Service (OBS): adiciona o
# repositório home:rodrigosbrito:vega e instala os pacotes de lá — openSUSE
# Leap 16.0. Alternativa: scripts/install.sh, que baixa RPM pré-compilado
# direto da release do GitHub, sem precisar de repositório nenhum.
#
# Uso:
#   curl -fsSL https://raw.githubusercontent.com/lyra-os-linux/vega/main/scripts/install-obs.sh | sudo bash
#
# VEGA_CLI_ONLY=1 sudo -E bash install-obs.sh   # só vegad + vega-cli, sem
#                                                # a interface GTK (e sem
#                                                # puxar gtk4/libadwaita) —
#                                                # pensado pra servidor
#                                                # headless administrado só
#                                                # por SSH.
set -euo pipefail

VEGA_OBS_PROJECT="home:rodrigosbrito:vega"
VEGA_OBS_REPO_URL="https://download.opensuse.org/repositories/home:/rodrigosbrito:/vega/openSUSE_Leap_16.0/"
VEGA_OBS_ALIAS="vega-obs"
VEGA_CLI_ONLY="${VEGA_CLI_ONLY:-0}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo bash install-obs.sh, ou via curl ... | sudo bash)." >&2
  exit 1
fi

if ! command -v zypper >/dev/null 2>&1; then
  echo "Erro: 'zypper' não encontrado — este script só cobre openSUSE Leap (via OBS)." >&2
  exit 1
fi

echo "==> Repositório $VEGA_OBS_PROJECT (OBS)"
if zypper lr "$VEGA_OBS_ALIAS" >/dev/null 2>&1; then
  echo "Já configurado como '$VEGA_OBS_ALIAS', seguindo com refresh."
else
  echo "Adicionando como '$VEGA_OBS_ALIAS'..."
  zypper --non-interactive addrepo "$VEGA_OBS_REPO_URL" "$VEGA_OBS_ALIAS"
fi

# --gpg-auto-import-keys: a Home Project do OBS não tem uma chave assinada
# por uma CA reconhecida — sem essa flag, o primeiro refresh pararia num
# prompt interativo pedindo pra confiar (ou não) na chave nova.
echo "==> Confiando na chave de assinatura do OBS (se for a primeira vez) e atualizando"
zypper --non-interactive --gpg-auto-import-keys refresh "$VEGA_OBS_ALIAS"

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
