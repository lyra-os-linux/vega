#!/usr/bin/env bash
# Shared by the reviewed-checkout installers; no action when sourced.
vega_obs_target() (
  local release_file="${1:-/etc/os-release}"
  local base_file="${2:-/usr/lib/os-release}"
  local ID="" VERSION_ID=""
  [ -r "$release_file" ] || return 1
  . "$release_file"
  if [ "$ID" = "lyra-os" ]; then
    [ -r "$base_file" ] || return 1
    ID="" VERSION_ID=""
    . "$base_file"
  fi
  case "$ID:$VERSION_ID" in
    opensuse-leap:16.0|opensuse-leap:16.1) printf 'openSUSE_Leap_%s\n' "$VERSION_ID" ;;
    *) echo "Base não suportada pelo instalador OBS: $ID $VERSION_ID." >&2; return 1 ;;
  esac
)

vega_obs_configure() {
  local project="$1" target="$2" alias url details
  case "$project" in vega|lyra) ;; *) return 1 ;; esac
  case "$target" in openSUSE_Leap_16.0|openSUSE_Leap_16.1) ;; *) return 1 ;; esac
  alias="$project-obs"
  url="https://download.opensuse.org/repositories/home:/rodrigosbrito:/$project/$target/"
  if details="$(zypper --xmlout repos "$alias" 2>/dev/null)"; then
    if [[ "$details" != *"<url>$url</url>"* && "$details" != *"<url>${url%/}</url>"* ]]; then
      echo "O repositório '$alias' já existe com outro endereço. Corrija-o para $url antes de instalar." >&2
      return 1
    fi
    zypper --non-interactive modifyrepo --enable --refresh --gpgcheck "$alias"
  else
    zypper --non-interactive addrepo --refresh "$url" "$alias"
  fi
  zypper --non-interactive --gpg-auto-import-keys refresh "$alias"
}
