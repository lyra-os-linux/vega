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
      { name: "VS Code", origin: "official", id: "vscode-repo", text: N_("Registra o repositório da Microsoft; depois instale o pacote code") },
      { name: "IntelliJ IDEA Community", origin: "flathub", id: "com.jetbrains.IntelliJ-IDEA-Community", text: N_("IDE para Java e Kotlin, de código aberto") },
      { name: "Rider", origin: "flathub", id: "com.jetbrains.Rider", text: N_("IDE para .NET e C#; grátis para uso não comercial") },
      { name: "Lazarus", origin: "official", id: "lazarus", text: N_("IDE visual para Free Pascal, parecida com o Delphi") },
      { name: "Eclipse", origin: "flathub", id: "org.eclipse.Java", text: N_("IDE para Java") },
      { name: "Meld", origin: "official", id: "meld", logo: "org.gnome.meld", text: N_("Compara arquivos e pastas lado a lado") },
    ],
  },
  {
    id: "git",
    title: N_("Git"),
    apps: [
      { name: "GitHub CLI", origin: "official", id: "gh", text: N_("Pull requests, issues e repositórios pelo terminal") },
      { name: "Git LFS", origin: "official", id: "git-lfs", text: N_("Arquivos grandes em repositórios Git") },
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
