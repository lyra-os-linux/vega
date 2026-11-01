// Aplicativos populares, curados a partir das perguntas frequentes da
// categoria "Softwares populares" (lyraos-desktop, Discussions #273–#283,
// #355, #391, #442–#480). Só entram apps instaláveis com um clique: pacote
// oficial (`origin: "official"`, nome do RPM) ou Flathub (`origin: "flathub"`,
// id do app). Pacote que só existe no repositório do fabricante leva `repo`
// (o Edge): o cartão adiciona o repositório, com a chave aprovada pelo
// usuário, antes de instalar.
//
// `logo` é o id do app no Flathub; o arquivo fica em ui/img/popular/<logo>.png
// (baixado por scripts/update-popular-logos.py). Só levam logo os projetos
// comunitários com ícone em licença livre. Marcas de empresas ou com política
// de marca restritiva (Spotify, Discord, Steam, Mozilla, Google etc.) ficam
// sem `logo` e mostram o ícone genérico com a inicial do nome: decisão do
// mantenedor em 31/10/2026, para evitar problema de marca registrada.

import { N_ } from "../../i18n.js";

export const POPULAR = [
  {
    // Discussão #273. Chrome, Brave e Opera vêm do Flathub: os repositórios
    // deles não publicam a chave junto (repomd.xml.key), e o Vega só confia
    // em chave aprovada pelo usuário. O Edge publica, e segue o fluxo
    // "adicionar repositório + aprovar chave" do .NET.
    id: "browsers",
    title: N_("Navegadores"),
    apps: [
      { name: "Firefox", origin: "official", id: "MozillaFirefox", text: N_("Navegador da Mozilla; já vem no Lyra") },
      { name: "Google Chrome", origin: "flathub", id: "com.google.Chrome", text: N_("Navegador do Google") },
      { name: "Microsoft Edge", origin: "official", id: "microsoft-edge-stable", repo: { name: "microsoft-edge", url: "https://packages.microsoft.com/yumrepos/edge" }, text: N_("Navegador da Microsoft, do repositório da Microsoft") },
      { name: "Brave", origin: "flathub", id: "com.brave.Browser", text: N_("Navegador com bloqueio de anúncios") },
      { name: "Opera", origin: "flathub", id: "com.opera.Opera", text: N_("Navegador com VPN e mensageiros na barra lateral") },
      { name: "Vivaldi", origin: "flathub", id: "com.vivaldi.Vivaldi", text: N_("Navegador personalizável") },
      { name: "Chromium", origin: "official", id: "chromium", text: N_("Navegador de código aberto") },
    ],
  },
  {
    id: "chat",
    title: N_("Conversa e e-mail"),
    apps: [
      { name: "Telegram", origin: "official", id: "telegram-desktop", text: N_("Mensagens rápidas, grupos e canais") },
      { name: "Discord", origin: "official", id: "discord", text: N_("Voz, vídeo e texto para comunidades") },
      { name: "Signal", origin: "flathub", id: "org.signal.Signal", text: N_("Mensagens com criptografia de ponta a ponta") },
      { name: "ZapZap", origin: "flathub", id: "com.rtosta.zapzap", text: N_("WhatsApp Web num aplicativo (da comunidade)") },
      { name: "Zoom", origin: "flathub", id: "us.zoom.Zoom", text: N_("Reuniões por vídeo") },
      { name: "Slack", origin: "flathub", id: "com.slack.Slack", text: N_("Conversa para equipes de trabalho") },
      { name: "Thunderbird", origin: "official", id: "MozillaThunderbird", text: N_("E-mail, calendário e contatos") },
    ],
  },
  {
    id: "media",
    title: N_("Música e vídeo"),
    apps: [
      { name: "Spotify", origin: "flathub", id: "com.spotify.Client", text: N_("Streaming de música e podcasts") },
      { name: "Stremio", origin: "flathub", id: "com.stremio.Stremio", text: N_("Central de filmes e séries") },
      { name: "Celluloid", origin: "official", id: "celluloid", logo: "io.github.celluloid_player.Celluloid", text: N_("Player de vídeo simples, baseado no mpv") },
      { name: "Rhythmbox", origin: "official", id: "rhythmbox", logo: "org.gnome.Rhythmbox3", text: N_("Biblioteca de músicas, rádios e podcasts") },
      { name: "Strawberry", origin: "official", id: "strawberry", logo: "org.strawberrymusicplayer.strawberry", text: N_("Player para coleções grandes de músicas") },
      { name: "Amberol", origin: "flathub", id: "io.bassi.Amberol", logo: "io.bassi.Amberol", text: N_("Toca uma pasta ou um álbum, sem biblioteca") },
      { name: "Easy Effects", origin: "flathub", id: "com.github.wwmm.easyeffects", logo: "com.github.wwmm.easyeffects", text: N_("Equalizador e efeitos para o som e o microfone") },
    ],
  },
  {
    id: "creation",
    title: N_("Vídeo, áudio e gravação"),
    apps: [
      { name: "OBS Studio", origin: "flathub", id: "com.obsproject.Studio", logo: "com.obsproject.Studio", text: N_("Gravação de tela e transmissão ao vivo") },
      { name: "Kdenlive", origin: "official", id: "kdenlive", logo: "org.kde.kdenlive", text: N_("Edição de vídeo") },
      { name: "Shotcut", origin: "official", id: "shotcut", logo: "org.shotcut.Shotcut", text: N_("Edição de vídeo") },
      { name: "Audacity", origin: "official", id: "audacity", text: N_("Gravação e edição de áudio") },
      { name: "HandBrake", origin: "flathub", id: "fr.handbrake.ghb", logo: "fr.handbrake.ghb", text: N_("Converte vídeos e reduz o tamanho") },
    ],
  },
  {
    id: "graphics",
    title: N_("Imagem, fotos e 3D"),
    apps: [
      // ArtCraft: suíte livre (MIT/Apache) de alternativas ao Adobe, em fase
      // alfa, instalada pelo ArtCraft Launcher. O Vega instala o AppImage
      // oficial da release mais recente, conferido pelo SHA256SUMS da release
      // (backend/appimage_apps.rs); o launcher se atualiza sozinho.
      { name: "ArtCraft", origin: "appimage", id: "artcraft-launcher", text: N_("Suíte livre de alternativas ao Adobe (PhotoCraft, VectorCraft, PdfCraft e outros); em fase alfa") },
      { name: "GIMP", origin: "official", id: "gimp", logo: "org.gimp.GIMP", text: N_("Edição de imagens") },
      { name: "Inkscape", origin: "official", id: "inkscape", logo: "org.inkscape.Inkscape", text: N_("Desenho vetorial") },
      { name: "Krita", origin: "official", id: "krita", logo: "org.kde.krita", text: N_("Pintura digital e ilustração") },
      { name: "Blender", origin: "official", id: "blender", logo: "org.blender.Blender", text: N_("Modelagem 3D, animação e renderização") },
      { name: "darktable", origin: "official", id: "darktable", logo: "org.darktable.Darktable", text: N_("Revelação de fotos RAW") },
      { name: "Shotwell", origin: "official", id: "shotwell", logo: "org.gnome.Shotwell", text: N_("Organiza fotos por data, com ajustes rápidos") },
      { name: "digiKam", origin: "official", id: "digikam", logo: "org.kde.digikam", text: N_("Coleções grandes de fotos, com etiquetas e rostos") },
      { name: "FreeCAD", origin: "flathub", id: "org.freecad.FreeCAD", logo: "org.freecad.FreeCAD", text: N_("Modelagem 3D paramétrica") },
      { name: "KiCad", origin: "official", id: "kicad", logo: "org.kicad.KiCad", text: N_("Projeto de circuitos eletrônicos") },
    ],
  },
  {
    id: "office",
    title: N_("Escritório e anotações"),
    apps: [
      { name: "ONLYOFFICE", origin: "flathub", id: "org.onlyoffice.desktopeditors", text: N_("Documentos, planilhas e apresentações") },
      { name: "Obsidian", origin: "flathub", id: "md.obsidian.Obsidian", text: N_("Anotações em Markdown, com links entre as notas") },
      { name: "Planify", origin: "flathub", id: "io.github.alainm23.planify", logo: "io.github.alainm23.planify", text: N_("Tarefas, lembretes e projetos") },
      { name: "Calibre", origin: "official", id: "calibre", logo: "com.calibre_ebook.calibre", text: N_("Biblioteca e conversão de e-books") },
      { name: "Okular", origin: "official", id: "okular", logo: "org.kde.okular", text: N_("Leitor de PDF com anotações") },
      { name: "Scribus", origin: "official", id: "scribus", logo: "net.scribus.Scribus", text: N_("Diagramação de revistas e folhetos") },
    ],
  },
  {
    id: "games",
    title: N_("Jogos"),
    apps: [
      { name: "Steam", origin: "flathub", id: "com.valvesoftware.Steam", text: N_("Loja e biblioteca de jogos") },
      { name: "Heroic", origin: "flathub", id: "com.heroicgameslauncher.hgl", logo: "com.heroicgameslauncher.hgl", text: N_("Jogos da Epic Games e da GOG") },
      { name: "Lutris", origin: "official", id: "lutris", logo: "net.lutris.Lutris", text: N_("Vários lançadores e emuladores num só lugar") },
      { name: "Bottles", origin: "flathub", id: "com.usebottles.bottles", logo: "com.usebottles.bottles", text: N_("Programas e jogos do Windows") },
      { name: "Prism Launcher", origin: "flathub", id: "org.prismlauncher.PrismLauncher", logo: "org.prismlauncher.PrismLauncher", text: N_("Minecraft") },
      { name: "ProtonUp-Qt", origin: "flathub", id: "net.davidotek.pupgui2", logo: "net.davidotek.pupgui2", text: N_("Versões extras do Proton e do Wine") },
    ],
  },
  {
    id: "utilities",
    title: N_("Utilitários"),
    apps: [
      { name: "Bitwarden", origin: "flathub", id: "com.bitwarden.desktop", text: N_("Gerenciador de senhas") },
      { name: "KeePassXC", origin: "official", id: "keepassxc", logo: "org.keepassxc.KeePassXC", text: N_("Cofre de senhas offline") },
      { name: "Dropbox", origin: "flathub", id: "com.dropbox.Client", text: N_("Sincroniza arquivos com a nuvem") },
      { name: "Nextcloud", origin: "official", id: "nextcloud-desktop", text: N_("Cliente de sincronização do Nextcloud") },
      { name: "qBittorrent", origin: "official", id: "qbittorrent", logo: "org.qbittorrent.qBittorrent", text: N_("Downloads por torrent") },
      { name: "AnyDesk", origin: "flathub", id: "com.anydesk.Anydesk", text: N_("Acesso remoto") },
      { name: "RustDesk", origin: "flathub", id: "com.rustdesk.RustDesk", text: N_("Acesso remoto de código aberto") },
      { name: "LinuxToys", origin: "official", id: "linuxtoys", logo: "linuxtoys", text: N_("Ajustes e instaladores prontos para o Linux, num só lugar") },
      { name: "GNOME Boxes", origin: "official", id: "gnome-boxes", logo: "org.gnome.Boxes", text: N_("Máquinas virtuais simples") },
      { name: "Mission Center", origin: "flathub", id: "io.missioncenter.MissionCenter", logo: "io.missioncenter.MissionCenter", text: N_("Monitor de CPU, memória, disco e GPU") },
      { name: "Extension Manager", origin: "flathub", id: "com.mattjakeman.ExtensionManager", logo: "com.mattjakeman.ExtensionManager", text: N_("Busca e instala extensões do GNOME") },
      { name: "Flatseal", origin: "flathub", id: "com.github.tchx84.Flatseal", logo: "com.github.tchx84.Flatseal", text: N_("Permissões dos aplicativos Flatpak") },
    ],
  },
];
