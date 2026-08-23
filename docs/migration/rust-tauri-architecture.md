# Arquitetura da migração para Tauri + Rust

Status: proposta para a issue #19. Ainda não é a arquitetura oficial — passa a
ser quando o scaffold da issue #22 comprovar que ela compila e abre.

## Objetivos e limites

A nova UI substitui `vega-gtk` (Rust, GTK4, libadwaita) por Rust + Tauri 2,
com o frontend em HTML/CSS/JS estático, leiaute e design system copiados do
projeto [`lyraos-desktop-welcome`](https://github.com/lyra-os-linux). O
`vegad` permanece em Go, ativado sob demanda, e os XMLs em `dbus/` continuam
sendo a fonte de verdade do contrato privilegiado. `vega-gtk` só é removido no
cutover (issue #41), depois que a matriz de paridade estiver `validado`.

Metas de aceite da milestone Tauri:

- paridade dos módulos hoje `implementado`/`validado` em
  `docs/migration/rust-gtk-parity.md`;
- nenhuma regressão de segurança: autorização continua exclusiva do `vegad` +
  polkit, segredos nunca tocam o processo do WebView;
- nenhum Node, npm ou Electron em tempo de execução do pacote final (Node só
  pode aparecer, se aparecer, como dependência de build);
- metas de desempenho definidas e aceitas explicitamente na issue #21, coletadas
  contra o scaffold real — esta issue não fixa números, só o método.

## Decisões

### Estrutura adotada

Novo membro do workspace `vega-tauri/`, ao lado de `vega-gtk/` até o cutover:

```text
vega-tauri/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── gen/schemas/           # capabilities.json etc., geradas pelo Tauri CLI
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── dbus/               # wrappers tauri::command sobre lyra-vega-dbus
│       └── commands/           # um módulo por domínio (software, backup, ...)
└── ui/
    ├── index.html
    ├── styles.css               # tokens copiados/adaptados do welcome
    ├── spacing.css
    ├── shell.js                 # sidebar, roteamento entre páginas, i18n
    └── pages/                   # um arquivo JS por módulo
```

Um único crate Rust (`src-tauri`) é suficiente no início, no mesmo espírito da
decisão equivalente do GTK: novos crates só são extraídos quando houver uma
fronteira testável real. O frontend não usa framework (React/Vue/etc.): segue
o padrão vanilla JS do `welcome`, coerente com o objetivo de não trazer uma
cadeia de build Node para o runtime.

O identificador da aplicação (`org.lyraos.Vega`) e o nome do binário
(`lyra-vega-gtk`) são preservados durante a migração. **Atenção**:
`lyraos-desktop-welcome/src-tauri/src/main.rs` chama `/usr/bin/vega-gtk`
diretamente (`open_vega`) para abrir o Vega a partir da tela de boas-vindas.
Enquanto o binário final não for renomeado, o pacote Tauri do Vega deve
instalar-se também como `/usr/bin/vega-gtk` (ou o `welcome` precisa ser
atualizado em lockstep) — decisão a fechar até o cutover (issue #41), nunca
depois dele, para não quebrar o primeiro login.

### Toolkit e estado

- `tauri` 2.x no backend. No Linux, `wry`/`tauri` embutem o WebView via
  `webkit2gtk-4.1` (API do port GTK3 da WebKitGTK, versionada "4.1" por
  compatibilidade histórica — **não** confundir com `webkitgtk-6.0`, o port
  GTK4, que `wry` ainda não suporta). O pacote de build correto no ambiente de
  referência (openSUSE Leap 16) é `webkit2gtk3-devel` (fornece
  `webkit2gtk-4.1.pc`); `webkit2gtk4-devel`, apesar do nome, fornece
  `webkitgtk-6.0.pc` e não serve para este build. `libwebkit2gtk-4_1-0`
  (runtime) já vem instalado por outras dependências do GNOME; o `-devel`
  precisa ser adicionado explicitamente ao ambiente de build e ao
  `BuildRequires` do pacote (ver issue #38), do mesmo jeito que
  `lyraos-desktop-welcome/packaging/lyra-welcome.spec` já declara
  `gtk3-devel`, `libsoup-devel` e `webkit2gtk3-devel`.
- Estado de domínio (dados vindos do `vegad`, seleção de pacote, etc.) vive no
  backend Rust ou em memória simples de página no JS; nenhuma lógica de
  autorização ou decisão de mutação vive só no frontend.
- Páginas reagem a estados explícitos: carregando, conteúdo, vazio, erro e
  backend indisponível — mesmo contrato de estados do `vega-gtk`.
- Leiaute: a topbar de marca, a paleta de tokens (claro/escuro) e o estilo de
  cartões do `welcome` são reaproveitados tal como estão; a navegação por
  `.page[hidden]` de wizard do `welcome` é substituída por uma sidebar
  persistente de 240px com busca de módulos, equivalente à do `vega-gtk`
  (`vega-gtk/src/ui/shell.rs`, `dock.rs`).

### IPC e assincronismo

- Comandos `tauri::command`, um módulo por domínio (`System`, `Software`,
  `Backup`, `Hardware`, `Snapshots`, `Services`, ...), chamando
  `lyra-vega-dbus` e devolvendo tipos `serde` já serializados para o frontend.
- Uma única conexão D-Bus compartilhada no processo backend, como no
  `vega-gtk`.
- Sinais do `vegad` (`Software.TransactionProgress`, `TransactionFinished`,
  `UpdatesAvailable`, progresso/alerta de Backup) chegam ao frontend como
  eventos Tauri (`app_handle.emit`), correlacionados por `transactionId`. Sem
  polling de conclusão pela UI.
- Toda chamada D-Bus, HTTP (Assistente de IA) ou leitura de processos roda em
  código assíncrono no backend; nenhuma trava a thread principal do Tauri nem
  o loop de eventos do WebView.
- Polling (quando existir, ex. status de rede) só roda enquanto a página
  correspondente estiver visível; o frontend cancela a assinatura ao trocar de
  página.

### Segurança e política de conteúdo

- CSP restritiva em `tauri.conf.json`, no espírito de
  `default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'`
  (a mesma base usada pelo `welcome`), sem origem remota nem `unsafe-inline`.
- `capabilities.json` concede o mínimo de permissões por janela; nenhuma
  capability de shell/fs genérica é habilitada — todo acesso a sistema passa
  pelos `tauri::command`s explícitos deste crate.
- Nenhuma chamada a D-Bus, `zypper`, `systemctl`, `nmcli` etc. a partir do
  JS/frontend; o frontend só invoca `tauri::command`s tipados.
- Autorização continua exclusivamente em `vegad` + polkit. Ocultar um botão ou
  desabilitar uma ação no frontend não é controle de acesso.
- Chaves de API do Assistente de IA continuam exclusivas do Secret Service via
  `secret-tool`, acessadas apenas pelo backend Rust; nunca serializadas para o
  frontend, `localStorage` ou `sessionStorage` do WebView.
- Senhas de Wi-Fi, tokens e chaves de IA não entram em logs nem em estado
  persistido em texto puro, no backend ou no frontend.

### i18n

Decisão de formato (JSON/JS próprio, como o `i18n.js` do `welcome`, versus
manter `gettext`/`.po`) fica a cargo da issue #26, que também migra as strings
existentes de `vega-gtk/po/`. Esta arquitetura só fixa a restrição: nenhuma
string de interface pode ficar hardcoded fora do mecanismo escolhido, e as
três locales atuais (pt-BR, en-US, es-ES) precisam de cobertura completa antes
do cutover.

### Compatibilidade e entrega

- O contrato `org.lyraos.Vega1` não muda por causa desta migração; qualquer
  extensão funcional segue as mesmas regras já descritas em
  `docs/migration/rust-gtk-architecture.md` (XML + polkit + testes).
- O crate compila no CI ao lado de `vega-gtk` durante toda a migração; os
  pacotes oficiais só passam a apontar para o Tauri no cutover.
- Empacotamento (issue #38) precisa confirmar a dependência de runtime do
  WebKitGTK e a ausência de Node/npm/Electron no pacote final.

## Portões de qualidade

Uma issue funcional desta milestone só está concluída quando possui:

1. comportamento de sucesso equivalente ao módulo correspondente em `vega-gtk`;
2. loading, vazio, falha e daemon indisponível;
3. confirmação para mutações de impacto;
4. teste do backend Rust (modelo/comandos) sem depender do WebView;
5. teste com cliente D-Bus mockado;
6. registro atualizado na matriz de paridade Tauri (issue #39);
7. verificação de teclado e foco no WebView.

## Próximas validações

O scaffold da issue #22 precisa confirmar, na prática: que o WebKitGTK do
ambiente de referência atende a CSP proposta sem downgrade de segurança, que o
tema claro/escuro do `welcome` acompanha `org.gnome.desktop.interface
color-scheme` como o `vega-gtk` já faz, e que a decisão de nome de binário
acima (`/usr/bin/vega-gtk`) não quebra a instalação lado a lado com o crate
GTK ainda presente no workspace.
