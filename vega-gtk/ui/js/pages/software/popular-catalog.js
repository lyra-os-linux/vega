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
      { name: "ZapZap", origin: "flathub", id: "com.rtosta.zapzap", tags: [N_("WhatsApp")], text: N_("WhatsApp Web num aplicativo (da comunidade)") },
      { name: "Zoom", origin: "flathub", id: "us.zoom.Zoom", tags: [N_("videoconferência"), N_("reunião online")], text: N_("Reuniões por vídeo") },
      { name: "Slack", origin: "flathub", id: "com.slack.Slack", text: N_("Conversa para equipes de trabalho") },
      { name: "Thunderbird", origin: "official", id: "MozillaThunderbird", tags: [N_("alternativa ao Outlook")], text: N_("E-mail, calendário e contatos") },
    ],
  },
  {
    id: "media",
    title: N_("Música e vídeo"),
    apps: [
      { name: "Spotify", origin: "flathub", id: "com.spotify.Client", text: N_("Streaming de música e podcasts") },
      { name: "Stremio", origin: "flathub", id: "com.stremio.Stremio", tags: [N_("filmes"), N_("séries")], text: N_("Central de filmes e séries") },
      { name: "Celluloid", origin: "official", id: "celluloid", logo: "io.github.celluloid_player.Celluloid", tags: [N_("mpv")], text: N_("Player de vídeo simples, baseado no mpv") },
      { name: "Rhythmbox", origin: "official", id: "rhythmbox", logo: "org.gnome.Rhythmbox3", text: N_("Biblioteca de músicas, rádios e podcasts") },
      { name: "Strawberry", origin: "official", id: "strawberry", logo: "org.strawberrymusicplayer.strawberry", text: N_("Player para coleções grandes de músicas") },
      { name: "Amberol", origin: "flathub", id: "io.bassi.Amberol", logo: "io.bassi.Amberol", text: N_("Toca uma pasta ou um álbum, sem biblioteca") },
      { name: "Easy Effects", origin: "flathub", id: "com.github.wwmm.easyeffects", logo: "com.github.wwmm.easyeffects", tasks: ["record"], text: N_("Equalizador e efeitos para o som e o microfone") },
    ],
  },
  {
    id: "creation",
    title: N_("Vídeo, áudio e gravação"),
    apps: [
      { name: "OBS Studio", origin: "flathub", id: "com.obsproject.Studio", logo: "com.obsproject.Studio", tasks: ["record"], tags: [N_("gravar a tela"), N_("transmissão ao vivo"), N_("live")], text: N_("Gravação de tela e transmissão ao vivo") },
      { name: "Kdenlive", origin: "official", id: "kdenlive", logo: "org.kde.kdenlive", tasks: ["edit"], tags: [N_("editor de vídeo")], text: N_("Edição de vídeo") },
      { name: "Shotcut", origin: "official", id: "shotcut", logo: "org.shotcut.Shotcut", tasks: ["edit"], tags: [N_("editor de vídeo")], text: N_("Edição de vídeo") },
      { name: "Audacity", origin: "official", id: "audacity", tasks: ["record", "edit"], tags: [N_("gravar áudio"), N_("podcast")], text: N_("Gravação e edição de áudio") },
      // FreeShow: só publica AppImage e RPM no GitHub, sem arquivo de
      // checksums; o Vega instala o AppImage 1.6.7, com SHA-256 fixado, na
      // conta do usuário (appimage_apps.rs).
      { name: "FreeShow", origin: "appimage", id: "freeshow", logo: "freeshow", tags: [N_("apresentações"), N_("projeção"), N_("igreja"), N_("letras de músicas")], text: N_("Apresentações e projeção para igrejas e eventos") },
      { name: "HandBrake", origin: "flathub", id: "fr.handbrake.ghb", logo: "fr.handbrake.ghb", tasks: ["edit"], tags: [N_("converter vídeo")], text: N_("Converte vídeos e reduz o tamanho") },
      { name: "SooperLooper", origin: "official", id: "sooperlooper", tasks: ["record"], tags: [N_("looper"), N_("loop ao vivo"), N_("JACK")], text: N_("Looper ao vivo: grava e sobrepõe camadas de áudio") },
    ],
  },
  {
    id: "graphics",
    title: N_("Imagem, fotos e 3D"),

    tasks: ["edit"],
    apps: [
      // ArtCraft: suíte livre (MIT/Apache) de alternativas ao Adobe, em fase
      // alfa, instalada pelo ArtCraft Launcher. O Vega instala o AppImage
      // oficial da release mais recente, conferido pelo SHA256SUMS da release
      // (backend/appimage_apps.rs); o launcher se atualiza sozinho.
      { name: "ArtCraft", origin: "appimage", id: "artcraft-launcher", text: N_("Suíte livre de alternativas ao Adobe (PhotoCraft, VectorCraft, PdfCraft e outros); em fase alfa") },
      { name: "GIMP", origin: "official", id: "gimp", logo: "org.gimp.GIMP", tags: [N_("alternativa ao Photoshop"), N_("editor de imagens")], text: N_("Edição de imagens") },
      { name: "Inkscape", origin: "official", id: "inkscape", logo: "org.inkscape.Inkscape", tags: [N_("alternativa ao Illustrator"), N_("vetor"), N_("SVG")], text: N_("Desenho vetorial") },
      { name: "Krita", origin: "official", id: "krita", logo: "org.kde.krita", tags: [N_("desenho"), N_("pintura digital")], text: N_("Pintura digital e ilustração") },
      { name: "Blender", origin: "official", id: "blender", logo: "org.blender.Blender", tags: [N_("modelagem 3D"), N_("animação")], text: N_("Modelagem 3D, animação e renderização") },
      { name: "darktable", origin: "official", id: "darktable", logo: "org.darktable.Darktable", tags: [N_("alternativa ao Lightroom"), N_("RAW")], text: N_("Revelação de fotos RAW") },
      { name: "Shotwell", origin: "official", id: "shotwell", logo: "org.gnome.Shotwell", text: N_("Organiza fotos por data, com ajustes rápidos") },
      { name: "digiKam", origin: "official", id: "digikam", logo: "org.kde.digikam", text: N_("Coleções grandes de fotos, com etiquetas e rostos") },
      { name: "FreeCAD", origin: "flathub", id: "org.freecad.FreeCAD", logo: "org.freecad.FreeCAD", tags: [N_("CAD"), N_("engenharia")], text: N_("Modelagem 3D paramétrica") },
      { name: "KiCad", origin: "official", id: "kicad", logo: "org.kicad.KiCad", tags: [N_("eletrônica"), N_("placa de circuito")], text: N_("Projeto de circuitos eletrônicos") },
    ],
  },
  {
    // vega#218: desenho e animação. Mesas digitalizadoras (Wacom, XP-Pen,
    // Huion) são configuradas pelo GNOME em Configurações → Mesa
    // digitalizadora; o xf86-input-wacom só serve à sessão X11.
    id: "drawing",
    title: N_("Desenho e animação"),
    tasks: ["edit"],
    apps: [
      { name: "MyPaint", origin: "flathub", id: "org.mypaint.MyPaint", logo: "org.mypaint.MyPaint", tags: [N_("pintura digital"), N_("mesa digitalizadora")], text: N_("Pintura com pincéis naturais, em tela infinita") },
      { name: "Pencil2D", origin: "flathub", id: "org.pencil2d.Pencil2D", logo: "org.pencil2d.Pencil2D", tags: [N_("animação 2D"), N_("desenho animado")], text: N_("Animação 2D quadro a quadro") },
      { name: "KColorChooser", origin: "official", id: "kcolorchooser", logo: "org.kde.kcolorchooser", tags: [N_("conta-gotas"), N_("paleta de cores")], text: N_("Seletor de cores, com conta-gotas na tela") },
      { name: "Skanlite", origin: "official", id: "skanlite", logo: "org.kde.skanlite", tags: [N_("scanner"), N_("digitalizar")], text: N_("Digitaliza desenhos e documentos no scanner") },
    ],
  },
  {
    // vega#219: complementa darktable, digiKam e Shotwell (Imagem, fotos e 3D).
    id: "photography",
    title: N_("Fotografia"),
    tasks: ["edit"],
    apps: [
      { name: "Hugin", origin: "official", id: "hugin", logo: "net.sourceforge.Hugin", tags: [N_("panorama")], text: N_("Junta várias fotos num panorama") },
      { name: "Entangle", origin: "official", id: "entangle", logo: "org.entangle_photo.Manager", tags: [N_("tethering"), N_("câmera no computador")], text: N_("Fotografa com a câmera ligada ao computador") },
      { name: "DisplayCAL", origin: "flathub", id: "net.displaycal.DisplayCAL", logo: "net.displaycal.DisplayCAL", tags: [N_("calibração de cor"), N_("colorímetro"), N_("perfil ICC")], text: N_("Calibra o monitor com colorímetro; o perfil é aplicado em Configurações → Cor") },
      { name: "PhotoCollage", origin: "flathub", id: "io.github.adrienverge.PhotoCollage", logo: "io.github.adrienverge.PhotoCollage", tags: [N_("colagem")], text: N_("Monta colagens de fotos") },
    ],
  },
  {
    // vega#220: o OpenLP tem muita demanda em igrejas no Brasil.
    id: "events",
    title: N_("Apresentações e eventos"),
    apps: [
      { name: "OpenLP", origin: "official", id: "OpenLP", tags: [N_("igreja"), N_("projeção"), N_("letras de músicas"), N_("culto")], text: N_("Projeta letras de músicas, versículos e slides em cultos e eventos") },
      { name: "Haruna", origin: "official", id: "haruna", logo: "org.kde.haruna", tags: [N_("reprodutor de vídeo")], text: N_("Reprodutor de vídeo para conferir e projetar") },
      { name: "Xjadeo", origin: "official", id: "xjadeo", tags: [N_("trilha sonora"), N_("Ardour")], tasks: ["edit"], text: N_("Mostra o vídeo sincronizado com o Ardour, para fazer a trilha") },
      { name: "QLC+", origin: "official", id: "qlcplus", tags: [N_("iluminação"), N_("DMX"), N_("mesa de luz")], text: N_("Controle de iluminação DMX para palcos e eventos") },
      { name: "K3b", origin: "official", id: "k3b", tags: [N_("gravar CD"), N_("gravar DVD")], text: N_("Grava CDs e DVDs") },
    ],
  },
  {
    // vega#221: Scribus e Calibre ficam em Escritório e anotações.
    id: "publishing",
    title: N_("Publicação e escrita"),
    tasks: ["edit"],
    apps: [
      { name: "FontForge", origin: "official", id: "fontforge", logo: "org.fontforge.FontForge", tags: [N_("criar fonte"), N_("tipografia")], text: N_("Cria e edita fontes") },
      { name: "gImageReader", origin: "flathub", id: "io.github.manisandro.gImageReader", logo: "io.github.manisandro.gImageReader", tags: [N_("OCR"), N_("PDF digitalizado")], text: N_("Extrai o texto de imagens e PDFs digitalizados (OCR); escolha o idioma Português no app") },
      { name: "PDF Arranger", origin: "official", id: "pdfarranger", logo: "com.github.jeromerobert.pdfarranger", tags: [N_("PDF"), N_("juntar PDF")], text: N_("Junta, separa e gira páginas de PDF") },
      { name: "Skribisto", origin: "flathub", id: "eu.skribisto.skribisto", logo: "eu.skribisto.skribisto", tags: [N_("escrever livro"), N_("roteiro")], tasks: ["edit", "study"], text: N_("Escrita de romances e roteiros, organizada por capítulos") },
    ],
  },
  {
    id: "office",
    title: N_("Escritório e anotações"),
    apps: [
      { name: "ONLYOFFICE", origin: "flathub", id: "org.onlyoffice.desktopeditors", tasks: ["edit", "study"], tags: [N_("alternativa ao Microsoft Office"), N_("Word"), N_("Excel"), N_("PowerPoint")], text: N_("Documentos, planilhas e apresentações") },
      { name: "WPS Office", origin: "flathub", id: "com.wps.Office", tasks: ["edit", "study"], tags: [N_("alternativa ao Microsoft Office"), N_("Word"), N_("Excel"), N_("PowerPoint"), N_("PDF")], text: N_("Documentos, planilhas, apresentações e PDF, com visual parecido com o do Office") },
      { name: "Obsidian", origin: "flathub", id: "md.obsidian.Obsidian", tasks: ["study"], tags: [N_("anotações"), N_("Markdown")], text: N_("Anotações em Markdown, com links entre as notas") },
      { name: "Planify", origin: "flathub", id: "io.github.alainm23.planify", logo: "io.github.alainm23.planify", tags: [N_("lista de tarefas")], text: N_("Tarefas, lembretes e projetos") },
      { name: "Calibre", origin: "official", id: "calibre", logo: "com.calibre_ebook.calibre", tasks: ["study"], tags: [N_("e-book"), N_("Kindle")], text: N_("Biblioteca e conversão de e-books") },
      { name: "Okular", origin: "official", id: "okular", logo: "org.kde.okular", tasks: ["study"], tags: [N_("PDF")], text: N_("Leitor de PDF com anotações") },
      { name: "Font Manager", origin: "official", id: "font-manager", logo: "com.github.FontManager.FontManager", tasks: ["edit"], tags: [N_("fontes"), N_("tipografia")], text: N_("Visualiza, compara, instala e organiza fontes") },
      { name: "Scribus", origin: "official", id: "scribus", logo: "net.scribus.Scribus", tasks: ["edit"], tags: [N_("diagramação"), N_("alternativa ao InDesign")], text: N_("Diagramação de revistas e folhetos") },
    ],
  },
  {
    id: "games",
    title: N_("Jogos"),

    tasks: ["play"],
    apps: [
      { name: "Steam", origin: "flathub", id: "com.valvesoftware.Steam", text: N_("Loja e biblioteca de jogos") },
      { name: "Heroic", origin: "flathub", id: "com.heroicgameslauncher.hgl", logo: "com.heroicgameslauncher.hgl", tags: [N_("Epic Games"), N_("GOG")], text: N_("Jogos da Epic Games e da GOG") },
      { name: "Lutris", origin: "official", id: "lutris", logo: "net.lutris.Lutris", text: N_("Vários lançadores e emuladores num só lugar") },
      { name: "Bottles", origin: "flathub", id: "com.usebottles.bottles", logo: "com.usebottles.bottles", text: N_("Programas e jogos do Windows") },
      { name: "Prism Launcher", origin: "flathub", id: "org.prismlauncher.PrismLauncher", logo: "org.prismlauncher.PrismLauncher", tags: [N_("Minecraft")], text: N_("Minecraft") },
      { name: "ProtonUp-Qt", origin: "flathub", id: "net.davidotek.pupgui2", logo: "net.davidotek.pupgui2", text: N_("Versões extras do Proton e do Wine") },
    ],
  },
  {
    id: "utilities",
    title: N_("Utilitários"),
    apps: [
      { name: "Bitwarden", origin: "flathub", id: "com.bitwarden.desktop", tags: [N_("senhas")], text: N_("Gerenciador de senhas") },
      { name: "KeePassXC", origin: "official", id: "keepassxc", logo: "org.keepassxc.KeePassXC", tags: [N_("senhas")], text: N_("Cofre de senhas offline") },
      { name: "Dropbox", origin: "flathub", id: "com.dropbox.Client", tags: [N_("nuvem"), N_("sincronizar arquivos")], text: N_("Sincroniza arquivos com a nuvem") },
      { name: "Nextcloud", origin: "official", id: "nextcloud-desktop", tags: [N_("nuvem"), N_("sincronizar arquivos")], text: N_("Cliente de sincronização do Nextcloud") },
      { name: "qBittorrent", origin: "official", id: "qbittorrent", logo: "org.qbittorrent.qBittorrent", tags: [N_("torrent")], text: N_("Downloads por torrent") },
      { name: "AnyDesk", origin: "flathub", id: "com.anydesk.Anydesk", tags: [N_("alternativa ao TeamViewer"), N_("acesso remoto")], text: N_("Acesso remoto") },
      { name: "RustDesk", origin: "flathub", id: "com.rustdesk.RustDesk", tags: [N_("acesso remoto")], text: N_("Acesso remoto de código aberto") },
      { name: "LinuxToys", origin: "official", id: "linuxtoys", logo: "linuxtoys", text: N_("Ajustes e instaladores prontos para o Linux, num só lugar") },
      { name: "GNOME Boxes", origin: "official", id: "gnome-boxes", logo: "org.gnome.Boxes", tags: [N_("máquina virtual")], text: N_("Máquinas virtuais simples") },
      { name: "Mission Center", origin: "flathub", id: "io.missioncenter.MissionCenter", logo: "io.missioncenter.MissionCenter", tags: [N_("gerenciador de tarefas")], text: N_("Monitor de CPU, memória, disco e GPU") },
      { name: "Extension Manager", origin: "flathub", id: "com.mattjakeman.ExtensionManager", logo: "com.mattjakeman.ExtensionManager", text: N_("Busca e instala extensões do GNOME") },
      { name: "Flatseal", origin: "flathub", id: "com.github.tchx84.Flatseal", logo: "com.github.tchx84.Flatseal", text: N_("Permissões dos aplicativos Flatpak") },
    ],
  },
];
