// Redes e TI (Vega → Software → Redes e TI): ferramentas para quem administra
// redes e dá suporte técnico. Mesmas regras de popular-catalog.js: só entra o
// que instala com um clique (pacote dos repositórios do Leap/Lyra ou app do
// Flathub), e só leva `logo` projeto comunitário com ícone em licença livre;
// marcas (AnyDesk, MikroTik, Termius…) usam o ícone genérico.

import { N_ } from "../../i18n.js";

export const NETWORK = [
  {
    id: "remote",
    title: N_("Acesso remoto"),
    apps: [
      { name: "Remmina", origin: "official", id: "remmina", logo: "org.remmina.Remmina", text: N_("Cliente de RDP, VNC e SSH") },
      { name: "Conexões", origin: "official", id: "gnome-connections", logo: "org.gnome.Connections", text: N_("Acesso remoto simples por RDP e VNC, do GNOME") },
      { name: "RustDesk", origin: "flathub", id: "com.rustdesk.RustDesk", text: N_("Suporte remoto de código aberto, alternativa ao TeamViewer") },
      { name: "AnyDesk", origin: "flathub", id: "com.anydesk.Anydesk", text: N_("Suporte remoto") },
      { name: "TigerVNC", origin: "official", id: "tigervnc", text: N_("Cliente e servidor VNC") },
      { name: "FreeRDP", origin: "official", id: "freerdp", text: N_("Cliente RDP de linha de comando") },
    ],
  },
  {
    id: "terminals",
    title: N_("Terminais e transferência"),
    apps: [
      { name: "PuTTY", origin: "official", id: "putty", text: N_("Cliente SSH, Telnet e serial") },
      { name: "Termius", origin: "flathub", id: "com.termius.Termius", text: N_("Cliente SSH com hosts sincronizados") },
      { name: "FileZilla", origin: "official", id: "filezilla", text: N_("Cliente FTP, FTPS e SFTP") },
      { name: "CuteCom", origin: "official", id: "cutecom", text: N_("Console serial para switches e roteadores") },
      { name: "Minicom", origin: "official", id: "minicom", text: N_("Console serial no terminal") },
    ],
  },
  {
    id: "equipment",
    title: N_("Equipamentos e VPN"),
    apps: [
      { name: "WinBox", origin: "flathub", id: "com.mikrotik.WinBox", text: N_("Administra roteadores MikroTik (versão nativa para Linux)") },
      { name: "WireGuard", origin: "official", id: "wireguard-tools", text: N_("Ferramentas da VPN WireGuard (wg e wg-quick)") },
      { name: "Net-SNMP", origin: "official", id: "net-snmp", text: N_("Consultas SNMP a equipamentos de rede") },
    ],
  },
  {
    id: "diagnostics",
    title: N_("Diagnóstico de rede"),
    apps: [
      { name: "Wireshark", origin: "official", id: "wireshark", text: N_("Análise de tráfego de rede") },
      { name: "Nmap", origin: "official", id: "nmap", text: N_("Varredura de hosts e portas") },
      { name: "Zenmap", origin: "official", id: "zenmap", text: N_("Interface gráfica do Nmap") },
      { name: "Angry IP Scanner", origin: "flathub", id: "org.angryip.ipscan", logo: "org.angryip.ipscan", text: N_("Descobre os aparelhos da rede") },
      { name: "MTR", origin: "official", id: "mtr-gtk", text: N_("Ping e traceroute juntos, com perda por salto") },
      { name: "iperf3", origin: "official", id: "iperf", text: N_("Mede a velocidade entre dois computadores") },
      { name: "Speedtest", origin: "official", id: "speedtest-cli", text: N_("Teste de velocidade da internet no terminal") },
      { name: "tcpdump", origin: "official", id: "tcpdump", text: N_("Captura de pacotes no terminal") },
      { name: "arp-scan", origin: "official", id: "arp-scan", text: N_("Lista os aparelhos da rede local pelo ARP") },
      { name: "dig e nslookup", origin: "official", id: "bind-utils", text: N_("Consultas de DNS") },
      { name: "whois", origin: "official", id: "whois", text: N_("Dono de domínios e blocos de IP") },
      { name: "Wavemon", origin: "official", id: "wavemon", text: N_("Sinal e canais do Wi-Fi no terminal") },
    ],
  },
  {
    id: "traffic",
    title: N_("Tráfego e monitoramento"),
    apps: [
      { name: "EtherApe", origin: "official", id: "etherape", text: N_("Mapa gráfico de quem conversa na rede") },
      { name: "iftop", origin: "official", id: "iftop", text: N_("Banda por conexão, em tempo real") },
      { name: "NetHogs", origin: "official", id: "nethogs", text: N_("Banda por programa, em tempo real") },
      { name: "nload", origin: "official", id: "nload", text: N_("Tráfego de entrada e saída por interface") },
    ],
  },
  {
    id: "support",
    title: N_("Suporte e manutenção"),
    apps: [
      { name: "Mission Center", origin: "flathub", id: "io.missioncenter.MissionCenter", logo: "io.missioncenter.MissionCenter", text: N_("Uso de processador, memória, disco e rede") },
      { name: "HardInfo2", origin: "official", id: "hardinfo2", text: N_("Inventário e testes de desempenho do hardware") },
      { name: "GSmartControl", origin: "official", id: "gsmartcontrol", text: N_("Saúde dos discos (SMART)") },
      { name: "GParted", origin: "official", id: "gparted", text: N_("Partições de discos e pendrives") },
      { name: "TestDisk e PhotoRec", origin: "official", id: "testdisk", text: N_("Recupera partições e arquivos apagados") },
      { name: "stress-ng", origin: "official", id: "stress-ng", text: N_("Teste de estresse de processador e memória") },
      { name: "KeePassXC", origin: "official", id: "keepassxc", logo: "org.keepassxc.KeePassXC", text: N_("Cofre de senhas da equipe, offline") },
    ],
  },
];
