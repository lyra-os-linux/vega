#!/usr/bin/env bash
# Desinstalador de conveniência: reverte scripts/install.sh removendo os
# pacotes do Vega (vega-gtk, vegad, vega-cli — cada um com seu próprio
# repositório e .spec desde a quebra do monorepo) via zypper. Os
# %preun/%postun dos .spec já cuidam de parar/desabilitar units systemd e
# recarregar D-Bus — este script só decide quais pacotes remover.
#
# Uso:
#   sudo bash scripts/uninstall.sh
#
# VEGA_PURGE=1 sudo -E bash scripts/uninstall.sh   # além dos pacotes, remove
#                                                   # estado que nenhum dos
#                                                   # dois lados (pacote nem
#                                                   # scriptlet) apaga:
#                                                   # /etc/vega (configs/senhas
#                                                   # do módulo Backup) e
#                                                   # /var/log/vega (export do
#                                                   # journal).
set -euo pipefail

VEGA_PURGE="${VEGA_PURGE:-0}"
PACKAGES=(vega-gtk vegad vega-cli)

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root (sudo bash scripts/uninstall.sh)." >&2
  exit 1
fi

distro_id=""
distro_id_like=""
if [ -r /etc/os-release ]; then
  . /etc/os-release
  distro_id="${ID:-}"
  distro_id_like="${ID_LIKE:-}"
fi

# purge_leftover_state remove diretórios de estado que sobrevivem à remoção
# do pacote em qualquer distro: são criados em runtime pelo próprio vegad
# (/etc/vega/backup, ver vegad/internal/dbusserver/backup.go) ou por
# systemd-tmpfiles (/var/log/vega, packaging/vegad/tmpfiles.d/vega-log.conf)
# — nenhum gerenciador de pacotes rastreia esses caminhos como parte do
# pacote, então "remove"/"purge" do pacote nunca os apaga sozinho.
purge_leftover_state() {
  [ "$VEGA_PURGE" = "1" ] || return 0
  echo "==> VEGA_PURGE=1: removendo estado remanescente do Vega"

  # Cada config de backup gera sua própria vega-backup-<id>.{service,timer,path}
  # em /etc/systemd/system (vegad/internal/dbusserver/backup.go,
  # writeBackupSystemdUnits) — nada disso é arquivo de pacote, então some
  # antes de apagar /etc/vega/backup pra não deixar timer/path apontando
  # pra config que não existe mais.
  shopt -s nullglob
  backup_units=(/etc/systemd/system/vega-backup-*.{service,timer,path})
  shopt -u nullglob
  if [ "${#backup_units[@]}" -gt 0 ]; then
    echo "==> Desabilitando e removendo unidades de backup (vega-backup-*)"
    for unit in "${backup_units[@]}"; do
      systemctl disable --now "$(basename "$unit")" 2>/dev/null || true
      rm -f "$unit"
    done
  fi

  # /etc/vega/backup guarda config/senha do restic em texto (backup.go,
  # backupStateDirDefault) — sensível, sempre no purge. Respeita
  # VEGA_BACKUP_STATE_DIR se o admin tiver movido o diretório de estado.
  rm -rf "${VEGA_BACKUP_STATE_DIR:-/etc/vega/backup}"
  rmdir --ignore-fail-on-non-empty /etc/vega 2>/dev/null || true
  rm -rf /var/log/vega

  systemctl daemon-reload
}

case "$distro_id $distro_id_like" in
  *opensuse*|*suse*)
    if ! command -v zypper >/dev/null 2>&1; then
      echo "Erro: 'zypper' não encontrado — isso não parece ser openSUSE." >&2
      exit 1
    fi

    installed=()
    for pkg in "${PACKAGES[@]}"; do
      rpm -q "$pkg" >/dev/null 2>&1 && installed+=("$pkg")
    done
    if [ "${#installed[@]}" -eq 0 ]; then
      echo "Nenhum pacote do Vega está instalado (rpm/zypper)."
    else
      echo "==> Removendo via zypper: ${installed[*]}"
      zypper --non-interactive remove -y "${installed[@]}"
    fi
    ;;
  *)
    echo "Distro não reconhecida (ID=$distro_id, ID_LIKE=$distro_id_like)." >&2
    echo "Este desinstalador cobre só openSUSE Leap." >&2
    exit 1
    ;;
esac

purge_leftover_state

cat <<EOF

Desinstalação concluída.
EOF

if [ "$VEGA_PURGE" != "1" ]; then
  cat <<EOF
Estado do Vega em /etc/vega e /var/log/vega (se existir) não foi removido —
rode de novo com VEGA_PURGE=1 se quiser apagar também esses diretórios.
EOF
fi

cat <<EOF
Nota: dados por usuário em ~/.local/share/vega-gtk/ai-settings.json (config
do assistente de IA da interface gráfica, possivelmente com chave de API, um
diretório por usuário que já usou o Vega) não são tocados por este script —
remova manualmente em cada \$HOME se quiser limpar também esses arquivos.
EOF
