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
`pt-BR`, `es-ES` e `zh-CN`. O idioma é resolvido automaticamente a cada
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
