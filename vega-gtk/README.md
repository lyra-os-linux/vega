# Vega GTK

Interface oficial do Vega, implementada em Rust, GTK4 e libadwaita. O `vegad`
e o contrato em `../dbus/` formam a fronteira privilegiada do aplicativo. O
pacote e o binário usam o nome `vega-gtk`.

## Dependências no openSUSE Leap

```bash
sudo zypper install rust cargo gtk4-devel libadwaita-devel
```

Quem usa `rustup` pode omitir os pacotes `rust` e `cargo`. O MSRV declarado no
manifest é Rust 1.92, exigido pela geração atual de bindings.

## Desenvolvimento

```bash
cargo run --manifest-path vega-gtk/Cargo.toml
cargo test --manifest-path vega-gtk/Cargo.toml
cargo clippy --manifest-path vega-gtk/Cargo.toml --all-targets -- -D warnings
```

O application ID oficial é `org.lyraos.Vega`.

## Personalização

A tela usa cartões em até duas colunas, seguindo a organização do Vega Qt.
Tema, bloqueio de tela, perfil, menu e dock abrem as páginas do Vega.
Papel de Parede e Cores abrem o painel Aparência das Configurações do GNOME; ícones,
fontes, cursores e janelas abrem o aplicativo Ajustes do GNOME na seção correspondente:
ícones e cursores em Aparência, fontes em Fontes e janelas em Janelas.

Há também atalhos para Configurações, Ajustes e Extensões do GNOME. Os atalhos
usam os aplicativos registrados na sessão e indicam quando um aplicativo
opcional não está instalado. O botão Atualizar relê os valores e a disponibilidade.
Menu e dock dependem do perfil Lyra com a extensão Sheliak ativa.
A troca de tema no Vega altera apenas a preferência claro/escuro do GNOME.
Ela preserva o papel de parede e o tema de ícones escolhidos. Os controles do
Vega acompanham a cor de destaque do GNOME; os estilos GTK e Shell vêm do GNOME.

No Ajustes 46, a seleção de seção usa AT-SPI, pois o aplicativo não oferece
um parâmetro de navegação. O auxiliar Python é incorporado ao binário e requer
PyGObject e o typelib Atspi 2.0 (dependências declaradas nos RPMs). Ele identifica
o processo do Ajustes pelo D-Bus, traduz os nomes das abas com o catálogo do
próprio aplicativo e seleciona a linha da barra lateral, inclusive em uma janela
já aberta. A operação tem tempo limite e informa falhas na tela do Vega.

## D-Bus

O módulo `src/dbus` acessa diretamente o system bus por `zbus`. A interface
`SystemClient` separa a UI do transporte e possui `MockSystemClient` para
testes sem daemon ou privilégios. Os testes de contrato leem os XMLs em
`../dbus/`; divergências de nomes ou assinaturas devem falhar no CI.

Software e Backup expõem `SoftwareEventStream` e `BackupEventStream`. Cada
chamada a `next()` aguarda todos os sinais da interface sem polling e devolve
um evento de domínio tipado; descartar o stream remove as subscriptions D-Bus.

## Internacionalização

A interface possui catálogos completos para `en-US` (padrão),
`pt-BR`, `en-US` e `es-ES`. O idioma é resolvido automaticamente a cada
abertura. A preferência registrada pelo GNOME AccountsService tem prioridade;
se ela não estiver disponível, são consultados `LC_ALL`, `LC_MESSAGES` e
`LANG`. Locales portáteis (`C` e `POSIX`) não escondem o idioma da sessão. Não
há seletor nem preferência paralela, e idiomas ausentes, malformados ou não
suportados usam inglês americano.

`build.rs` exige `msgfmt` e compila os quatro arquivos `po/<lang>.po` para
`po/locale/<locale>/LC_MESSAGES/vega-gtk.mo`. Para testar um catálogo localmente
sem instalar o pacote:

```bash
LANG=en_US.UTF-8 cargo run --manifest-path vega-gtk/Cargo.toml
```

Atualizar o template depois de mexer nas strings requer `xtr`, que entende
sintaxe Rust. O script de manutenção também valida placeholders; traduções
geradas por ele são rascunhos e exigem revisão humana:

```bash
cargo install xtr
python3 ../scripts/update-vega-gtk-translations.py
```
