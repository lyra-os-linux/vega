// Ferramentas e linguagens de programação, curadas a partir das perguntas
// frequentes da categoria "Desenvolvedores" (lyraos-desktop, Discussions
// #281–#296 e #345). Mesmas regras de popular-catalog.js: só entra o que
// instala com um clique (um pacote oficial ou um app do Flathub), e só leva
// `logo` projeto comunitário com ícone em licença livre. Linguagens (Python,
// Rust, Go, Java…) e marcas de empresas têm política de marca própria e usam
// o ícone genérico.
//
// Cada cartão instala um pacote; quando a ferramenta depende de outro (npm24
// → nodejs24, php-composer2 → php8, java-*-devel → JRE), o cartão aponta para
// o pacote que puxa o resto. `alternatives` lista pacotes que já entregam a
// mesma ferramenta e conflitam com o do cartão (o rustup não convive com o
// rust/cargo do sistema): com um deles instalado, o cartão mostra "Instalado".
// `repo` marca pacote que só existe no repositório do fabricante (o .NET da
// Microsoft, #291): o cartão adiciona o repositório, com a chave aprovada pelo
// usuário, antes de instalar.

import { N_ } from "../../i18n.js";

export const DEVELOPER = [
  {
    id: "languages",
    title: N_("Linguagens"),
    apps: [
      { name: "C/C++ (GCC)", origin: "official", id: "gcc-c++", text: N_("Compiladores GCC de C e C++") },
      { name: "Clang", origin: "official", id: "clang", text: N_("Compilador C e C++ do LLVM") },
      { name: "Python 3.14", origin: "official", id: "python314", text: N_("Versão mais nova, ao lado do Python 3.13 do sistema") },
      { name: "Rust", origin: "official", id: "rustup", alternatives: ["rust", "cargo"], text: N_("Instala e atualiza o Rust e o cargo") },
      { name: "Go", origin: "official", id: "go", text: N_("Linguagem e ferramentas do Go") },
      { name: "Node.js 24 + npm", origin: "official", id: "npm24", text: N_("Node.js do sistema; o nvm já vem instalado") },
      { name: "Java 21 (OpenJDK)", origin: "official", id: "java-21-openjdk-devel", text: N_("JDK com suporte longo (LTS)") },
      { name: "Java 25 (OpenJDK)", origin: "official", id: "java-25-openjdk-devel", text: N_("JDK mais recente com suporte longo (LTS)") },
      { name: "Ruby", origin: "official", id: "ruby-devel", text: N_("Ruby com os cabeçalhos para compilar gems") },
      { name: "PHP 8", origin: "official", id: "php8-cli", text: N_("PHP pela linha de comando") },
      { name: ".NET 8 SDK", origin: "official", id: "dotnet-sdk-8.0", repo: { name: "microsoft-prod", url: "https://packages.microsoft.com/opensuse/16/prod/" }, text: N_("C# e .NET com suporte longo (LTS), do repositório da Microsoft") },
      { name: ".NET 10 SDK", origin: "official", id: "dotnet-sdk-10.0", repo: { name: "microsoft-prod", url: "https://packages.microsoft.com/opensuse/16/prod/" }, text: N_("C# e .NET mais recente (LTS), do repositório da Microsoft") },
      { name: "Free Pascal", origin: "official", id: "fpc", text: N_("Compilador Pascal e Object Pascal") },
      { name: "Elixir", origin: "official", id: "elixir", text: N_("Linguagem funcional sobre a máquina do Erlang") },
      { name: "Erlang", origin: "official", id: "erlang", text: N_("Linguagem e plataforma OTP") },
      { name: "Haskell (GHC)", origin: "official", id: "ghc", text: N_("Compilador Glasgow Haskell") },
      { name: "Zig", origin: "official", id: "zig", text: N_("Linguagem de sistemas e compilador C") },
      { name: "Lua 5.4", origin: "official", id: "lua54", text: N_("Linguagem de script leve") },
    ],
  },
  {
    id: "build",
    title: N_("Compilação e depuração"),
    apps: [
      { name: "CMake", origin: "official", id: "cmake", text: N_("Gera os arquivos de compilação de projetos C e C++") },
      { name: "Meson", origin: "official", id: "meson", text: N_("Sistema de compilação rápido") },
      { name: "Ninja", origin: "official", id: "ninja", text: N_("Executor de compilação usado pelo CMake e pelo Meson") },
      { name: "GDB", origin: "official", id: "gdb", text: N_("Depurador") },
      { name: "Valgrind", origin: "official", id: "valgrind", text: N_("Encontra vazamentos e erros de memória") },
      { name: "clangd (clang-tools)", origin: "official", id: "clang-tools", text: N_("Autocompletar de C e C++ no Zed e no VS Code") },
      { name: "pip", origin: "official", id: "python313-pip", text: N_("Instalador de pacotes do Python") },
      { name: "pipx", origin: "official", id: "python313-pipx", text: N_("Ferramentas de Python isoladas (uv, poetry, ruff)") },
      { name: "Maven", origin: "official", id: "maven", text: N_("Compilação e dependências de projetos Java") },
      { name: "Composer", origin: "official", id: "php-composer2", text: N_("Dependências de projetos PHP") },
    ],
  },
  {
    id: "editors",
    title: N_("Editores e IDEs"),
    apps: [
      { name: "Zed", origin: "official", id: "zed", text: N_("Editor de código rápido; já vem no Lyra") },
      { name: "VS Code", origin: "official", id: "code", repo: { package: "vscode-repo", name: "code", fingerprint: "BC528686B50D79E339D3721CEB3E94ADBE1229CF" }, text: N_("Editor da Microsoft, do repositório oficial da Microsoft") },
      { name: "IntelliJ IDEA Community", origin: "flathub", id: "com.jetbrains.IntelliJ-IDEA-Community", text: N_("IDE para Java e Kotlin, de código aberto") },
      { name: "Rider", origin: "flathub", id: "com.jetbrains.Rider", text: N_("IDE para .NET e C#; grátis para uso não comercial") },
      // Demais IDEs da JetBrains no Flathub (empacotamento da comunidade,
      // como o IntelliJ e o Rider). O PyCharm Community saiu do Flathub
      // quando a JetBrains unificou o PyCharm; RubyMine, DataSpell e o
      // Toolbox não estão lá (a discussão #294 indica o Toolbox).
      { name: "IntelliJ IDEA Ultimate", origin: "flathub", id: "com.jetbrains.IntelliJ-IDEA-Ultimate", text: N_("IDE completa para Java, Kotlin e web; paga, com período de teste") },
      { name: "PyCharm", origin: "flathub", id: "com.jetbrains.PyCharm-Professional", text: N_("IDE para Python; uso básico gratuito, recursos Pro pagos") },
      { name: "WebStorm", origin: "flathub", id: "com.jetbrains.WebStorm", text: N_("IDE para JavaScript e TypeScript; grátis para uso não comercial") },
      { name: "CLion", origin: "flathub", id: "com.jetbrains.CLion", text: N_("IDE para C e C++; grátis para uso não comercial") },
      { name: "GoLand", origin: "flathub", id: "com.jetbrains.GoLand", text: N_("IDE para Go; paga, com período de teste") },
      { name: "RustRover", origin: "flathub", id: "com.jetbrains.RustRover", text: N_("IDE para Rust; grátis para uso não comercial") },
      { name: "PhpStorm", origin: "flathub", id: "com.jetbrains.PhpStorm", text: N_("IDE para PHP; paga, com período de teste") },
      { name: "DataGrip", origin: "flathub", id: "com.jetbrains.DataGrip", text: N_("IDE para bancos de dados e SQL; grátis para uso não comercial") },
      { name: "Lazarus", origin: "official", id: "lazarus", text: N_("IDE visual para Free Pascal, parecida com o Delphi") },
      { name: "Eclipse", origin: "flathub", id: "org.eclipse.Java", text: N_("IDE para Java") },
      { name: "Meld", origin: "official", id: "meld", logo: "org.gnome.meld", text: N_("Compara arquivos e pastas lado a lado") },
    ],
  },
  {
    // Os métodos seguem a documentação de cada fabricante,
    // adaptada ao openSUSE: Claude Code e Cursor pelos repositórios RPM
    // oficiais (pacotes claude-code-repo e cursor-repo, com a chave fixada);
    // Codex, Copilot, Gemini e opencode pelo npm na conta do usuário
    // (backend/npm_apps.rs, sem sudo); Ollama pelo pacote do openSUSE.
    id: "ai",
    title: N_("IA para desenvolvimento"),
    apps: [
      { name: "Claude Code (CLI)", origin: "official", id: "claude-code", repo: { package: "claude-code-repo", name: "claude-code", fingerprint: "31DDDE24DDFAB679F42D7BD2BAA929FF1A7ECACE" }, text: N_("Assistente de programação da Anthropic no terminal") },
      { name: "Codex CLI", origin: "npm", id: "@openai/codex", text: N_("Assistente de programação da OpenAI no terminal") },
      { name: "GitHub Copilot CLI", origin: "npm", id: "@github/copilot", text: N_("Copilot no terminal; exige uma assinatura do GitHub Copilot") },
      { name: "Gemini CLI", origin: "npm", id: "@google/gemini-cli", text: N_("Assistente do Google no terminal, para licenças do Gemini Code Assist") },
      { name: "opencode", origin: "npm", id: "opencode-ai", text: N_("Assistente de código aberto no terminal, com vários provedores de IA") },
      { name: "Cursor", origin: "official", id: "cursor", repo: { package: "cursor-repo", name: "cursor", fingerprint: "380FF4BCDC34A4BD92A3565342A1772E62E492D6" }, text: N_("Editor com IA, baseado no VS Code") },
      { name: "Ollama", origin: "official", id: "ollama", text: N_("Roda modelos de IA no próprio computador") },
      { name: "Alpaca", origin: "flathub", id: "com.jeffser.Alpaca", logo: "com.jeffser.Alpaca", text: N_("Conversa com modelos locais do Ollama, integrado ao GNOME") },
      { name: "LM Studio", origin: "flathub", id: "ai.lmstudio.lm-studio", text: N_("Baixa e roda modelos de IA locais") },
    ],
  },
  {
    // Arquitetura de software: modelagem, UML, C4 e diagramas como código.
    // Archi e Modelio não estão no Flathub nem nos repositórios do Leap.
    id: "architecture",
    title: N_("Arquitetura de software"),
    apps: [
      { name: "draw.io", origin: "flathub", id: "com.jgraph.drawio.desktop", text: N_("Diagramas de arquitetura, fluxos e infraestrutura") },
      { name: "Umbrello", origin: "official", id: "umbrello", logo: "org.kde.umbrello", text: N_("Modelagem UML, com geração de código") },
      { name: "PlantUML", origin: "official", id: "plantuml", text: N_("Diagramas UML e C4 escritos como texto") },
      { name: "Mermaid CLI", origin: "npm", id: "@mermaid-js/mermaid-cli", text: N_("Gera diagramas Mermaid em SVG e PNG pelo terminal (mmdc)") },
      { name: "Graphviz", origin: "official", id: "graphviz", text: N_("Gera grafos e diagramas a partir de texto (dot)") },
      { name: "Dia", origin: "official", id: "dia", text: N_("Editor de diagramas estruturados") },
    ],
  },
  {
    // Ferramentas de estação de trabalho para DevOps. kubectl, k9s, kind e
    // minikube não estão nos repositórios do Leap 16.1.
    id: "devops",
    title: N_("DevOps e infraestrutura"),
    apps: [
      { name: "Ansible", origin: "official", id: "ansible", text: N_("Automação de configuração de servidores") },
      { name: "OpenTofu", origin: "official", id: "opentofu", text: N_("Infraestrutura como código, compatível com o Terraform") },
      { name: "Helm", origin: "official", id: "helm", text: N_("Gerenciador de pacotes do Kubernetes") },
      { name: "Freelens", origin: "flathub", id: "app.freelens.Freelens", text: N_("IDE para clusters Kubernetes") },
      { name: "Headlamp", origin: "flathub", id: "io.kinvolk.Headlamp", text: N_("Painel do Kubernetes") },
      { name: "Podman Desktop", origin: "flathub", id: "io.podman_desktop.PodmanDesktop", text: N_("Interface gráfica para contêineres e Kubernetes") },
      { name: "Buildah", origin: "official", id: "buildah", text: N_("Constrói imagens de contêiner sem serviço") },
      { name: "Skopeo", origin: "official", id: "skopeo", text: N_("Copia e inspeciona imagens entre registros") },
      { name: "Trivy", origin: "official", id: "trivy", text: N_("Procura vulnerabilidades em imagens, código e IaC") },
      { name: "Cosign", origin: "official", id: "cosign", text: N_("Assina e verifica imagens de contêiner") },
      { name: "hadolint", origin: "official", id: "hadolint", text: N_("Revisa Dockerfiles") },
      { name: "act", origin: "official", id: "act", text: N_("Roda GitHub Actions localmente") },
      { name: "GitLab CLI (glab)", origin: "official", id: "glab", text: N_("Merge requests, pipelines e issues do GitLab pelo terminal") },
      { name: "jq", origin: "official", id: "jq", text: N_("Consulta e transforma JSON no terminal") },
      { name: "yq", origin: "official", id: "yq", text: N_("Consulta e transforma YAML no terminal") },
      { name: "Wireshark", origin: "official", id: "wireshark", text: N_("Análise de tráfego de rede") },
    ],
  },
  {
    id: "git",
    title: N_("Git"),
    apps: [
      { name: "GitHub CLI", origin: "official", id: "gh", text: N_("Pull requests, issues e repositórios pelo terminal") },
      { name: "Git LFS", origin: "official", id: "git-lfs", text: N_("Arquivos grandes em repositórios Git") },
      { name: "gitg", origin: "official", id: "gitg", logo: "org.gnome.gitg", text: N_("Histórico e commits do Git, integrado ao GNOME") },
      { name: "Git Cola", origin: "official", id: "git-cola", logo: "com.github.git_cola.git-cola", text: N_("Interface gráfica para o Git") },
      { name: "lazygit", origin: "official", id: "lazygit", text: N_("Interface do Git no terminal") },
      { name: "GitHub Desktop", origin: "flathub", id: "io.github.shiftey.Desktop", text: N_("GitHub Desktop para Linux, mantido pela comunidade") },
      { name: "Sublime Merge", origin: "flathub", id: "com.sublimemerge.App", text: N_("Cliente Git rápido; uso gratuito com aviso de licença") },
    ],
  },
  {
    id: "containers",
    title: N_("Contêineres"),
    apps: [
      { name: "Podman", origin: "official", id: "podman", text: N_("Contêineres sem serviço nem acesso de administrador") },
      { name: "Distrobox", origin: "official", id: "distrobox", text: N_("Ferramentas de outras distribuições num contêiner") },
      { name: "Docker", origin: "official", id: "docker", text: N_("Motor de contêineres Docker") },
      { name: "Docker Compose", origin: "official", id: "docker-compose", text: N_("Vários contêineres descritos num arquivo") },
      { name: "Docker Buildx", origin: "official", id: "docker-buildx", text: N_("Builds de imagens multiplataforma") },
    ],
  },
  {
    id: "databases",
    title: N_("Bancos de dados e APIs"),
    apps: [
      { name: "PostgreSQL 18", origin: "official", id: "postgresql18-server", text: N_("Servidor PostgreSQL") },
      { name: "MariaDB", origin: "official", id: "mariadb", text: N_("Servidor compatível com o MySQL") },
      { name: "Valkey", origin: "official", id: "valkey", text: N_("Banco em memória compatível com o Redis") },
      { name: "SQLite", origin: "official", id: "sqlite3", text: N_("Banco de dados num arquivo, pelo terminal") },
      { name: "Draco Postgres", origin: "official", id: "postgres-draco", logo: "postgres-draco", text: N_("Cliente PostgreSQL nativo, com editor SQL, diagrama ER e administração") },
      { name: "DBeaver", origin: "flathub", id: "io.dbeaver.DBeaverCommunity", text: N_("Cliente gráfico para vários bancos de dados") },
      { name: "Postman", origin: "flathub", id: "com.getpostman.Postman", text: N_("Testa e documenta APIs") },
      { name: "Bruno", origin: "flathub", id: "com.usebruno.Bruno", text: N_("Cliente de APIs de código aberto; coleções ficam no Git") },
      { name: "Insomnia", origin: "flathub", id: "rest.insomnia.Insomnia", text: N_("Cliente de APIs REST, GraphQL e gRPC") },
    ],
  },
  {
    id: "mobile",
    title: N_("Android"),
    apps: [
      { name: "Android Studio", origin: "flathub", id: "com.google.AndroidStudio", text: N_("IDE e SDK para apps Android") },
      { name: "Android Tools (adb)", origin: "official", id: "android-tools", text: N_("adb e fastboot para o celular por USB") },
    ],
  },
];
