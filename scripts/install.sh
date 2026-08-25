#!/usr/bin/env bash
# Instalador de conveniência: baixa os pacotes RPM publicados manualmente
# nas releases mais recentes do vega-gtk (este repo), vegad e vega-cli, e
# instala com zypper. Cobre só openSUSE Leap.
#
# Desde a quebra do monorepo, cada componente tem seu próprio repositório e
# sua própria release — este script busca o RPM de cada um separadamente,
# então trava todos na mesma tag (VEGA_VERSION) só funciona se as releases
# dos três repos usarem o mesmo esquema de versão.
#
# Uso a partir de um checkout revisado:
#   sudo bash scripts/install.sh
#
# VEGA_VERSION=v5.1.22 sudo -E bash install.sh   # trava numa tag específica
#                                                 # (mesma tag nos 3 repos)
# VEGA_CLI_ONLY=1 sudo -E bash install.sh        # só vegad + vega-cli, sem
#                                                 # a interface GTK (e sem
#                                                 # puxar gtk4/libadwaita) —
#                                                 # pensado pra servidor
#                                                 # headless administrado só
#                                                 # por SSH.
set -euo pipefail

VEGA_CLI_ONLY="${VEGA_CLI_ONLY:-0}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Rode como root a partir de um checkout revisado (sudo bash scripts/install.sh)." >&2
  exit 1
fi

distro_id=""
distro_id_like=""
if [ -r /etc/os-release ]; then
  . /etc/os-release
  distro_id="${ID:-}"
  distro_id_like="${ID_LIKE:-}"
fi

# download_release_assets baixa pra $workdir todo asset .rpm da release do
# repo passado, usando a API de releases do GitHub.
download_release_assets() {
  local repo="$1"
  local release_tag="${VEGA_VERSION:-latest}"
  local api_url
  if [ "$release_tag" = "latest" ]; then
    api_url="https://api.github.com/repos/$repo/releases/latest"
  else
    api_url="https://api.github.com/repos/$repo/releases/tags/$release_tag"
  fi

  echo "==> Consultando release ($release_tag) em $repo" >&2
  local release_json
  release_json="$(curl -fsSL "$api_url")"

  local urls=()
  mapfile -t urls < <(printf '%s' "$release_json" \
    | grep -Eo '"browser_download_url": *"[^"]*\.rpm"' \
    | sed -E 's/.*"(https:[^"]+)"/\1/' \
    | grep -Ev 'debuginfo|debugsource')

  if [ "${#urls[@]}" -eq 0 ]; then
    echo "Erro: nenhum asset '*.rpm' encontrado na release '$release_tag' de $repo." >&2
    echo "Confira se os RPMs foram publicados manualmente para essa tag:" >&2
    echo "  https://github.com/$repo/releases" >&2
    exit 1
  fi

  for url in "${urls[@]}"; do
    echo "==> Baixando $(basename "$url")" >&2
    curl -fsSL "$url" -o "$workdir/$(basename "$url")"
  done
}

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

case "$distro_id $distro_id_like" in
  *opensuse*|*suse*)
    if ! command -v zypper >/dev/null 2>&1; then
      echo "Erro: 'zypper' não encontrado — isso não parece ser openSUSE." >&2
      exit 1
    fi

    download_release_assets "lyra-os-linux/vegad"
    download_release_assets "lyra-os-linux/vega-cli"
    if [ "$VEGA_CLI_ONLY" != "1" ]; then
      download_release_assets "lyra-os-linux/vega"
    fi

    echo "==> Instalando via zypper"
    echo "Aviso: os RPMs desta release ainda não são assinados (sem chave GPG"
    echo "configurada), então a instalação usa --allow-unsigned-rpm."
    zypper --non-interactive install -y --allow-unsigned-rpm "$workdir"/*.rpm
    ;;
  *)
    echo "Distro não reconhecida (ID=$distro_id, ID_LIKE=$distro_id_like)." >&2
    echo "Este instalador cobre só openSUSE Leap." >&2
    exit 1
    ;;
esac

if [ "$VEGA_CLI_ONLY" = "1" ]; then
  cat <<EOF

Instalação concluída.
- Daemon: vegad, ativado sob demanda via D-Bus (org.lyraos.Vega1)
- Interface: /usr/bin/vega (terminal, dialog)

Empacotamento ainda é considerado de teste — reporte problemas nos
repositórios correspondentes (lyra-os-linux/vegad, lyra-os-linux/vega-cli,
lyra-os-linux/vega).
EOF
else
  cat <<EOF

Instalação concluída.
- Daemon: vegad, ativado sob demanda via D-Bus (org.lyraos.Vega1)
- Interface gráfica: /usr/bin/vega-gtk
- Interface de terminal: /usr/bin/vega (rode via SSH, sem precisar do ambiente gráfico)

Empacotamento ainda é considerado de teste — reporte problemas nos
repositórios correspondentes (lyra-os-linux/vegad, lyra-os-linux/vega-cli,
lyra-os-linux/vega).
EOF
fi
