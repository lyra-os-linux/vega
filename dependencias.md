# Dependências do sistema para o Vega

O `vegad` fala com o gerenciador de pacotes por trás de uma camada de
abstração (`vegad/internal/distro`) com um único backend hoje: Zypper em
openSUSE Leap. As dependências abaixo são as necessárias nessa distro.

## openSUSE Leap

Os RPMs são publicados manualmente no OBS e, quando necessário, em uma GitHub
Release. Também podem ser compilados localmente pelos specs em
`packaging/opensuse/`. O backend Zypper/hardware NVIDIA
(`vegad/internal/distro/zypper.go`, `hardware_opensuse.go`) ainda não foi
validado ponta a ponta num Leap real — trate os nomes de pacote abaixo como
ponto de partida, não garantia.

### Necessários só para compilar (o script verifica e aborta se faltar)

- `go`, `rust`, `cargo` e `gcc`
- `pkg-config`, `gtk4-devel` e `libadwaita-devel`

### Base de sistema (já presente em qualquer Leap com systemd/D-Bus/polkit; nada equivalente a `pacman`/`bluez` é obrigatório)

- `systemd`
- `dbus-1`
- `polkit`

### Pacotes opcionais (um por módulo — sem eles o módulo correspondente reporta "indisponível", o resto do app funciona normalmente; o script `install.sh` já avisa quais binários estão faltando)

| Pacote (zypper) | Binário verificado | Módulo |
| --- | --- | --- |
| `snapper` | `snapper` | Snapshots (config **`root`**, requer raiz em Btrfs) |
| `flatpak` | `flatpak` | Software (origem Flathub) |
| `NetworkManager` | `nmcli` | Rede |
| `restic` | `restic` | Backup |
| `firewalld` | `firewall-cmd` | Firewall (precisa do serviço ativo) |
| `fwupd` | `fwupdmgr` | Hardware (firmware) |
| `bluez` | `bluetoothctl` | Bluetooth |

### Resumo de instalação (openSUSE)

```sh
# dependências de build
sudo zypper install go rust cargo gcc pkg-config gtk4-devel libadwaita-devel

# opcionais (conforme os módulos desejados)
sudo zypper install snapper flatpak NetworkManager restic firewalld fwupd bluez
sudo systemctl enable --now firewalld

# instala a interface e o daemon a partir do checkout do repo
sudo packaging/opensuse/install.sh
```

## Requisitos de sistema

- **Barramento D-Bus system ativo** (`dbus.service`). O `vegad` é ativado
  sob demanda pelo D-Bus (`Type=dbus`, `BusName=org.lyraos.Vega1`, sem
  `[Install]` — não se usa `systemctl enable`). Isso só funciona se o pacote
  instalar `/usr/share/dbus-1/system-services/org.lyraos.Vega1.service` com
  `SystemdService=vegad.service`; sem esse arquivo o barramento não sabe
  que precisa pedir ao systemd para subir o daemon.
- **polkit ativo** — autoriza as ações privilegiadas que o `vegad` expõe
  (`org.lyraos.vega.policy`).
- Sistema de arquivos raiz em **Btrfs** e `snapper` instalado, para o módulo
  de Snapshots — sem snapper o menu "Pontos de Restauração" fica oculto.
