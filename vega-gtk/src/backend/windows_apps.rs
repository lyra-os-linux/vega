//! Apps do Windows (Vega → Software → Apps do Windows).
//!
//! Cada app é um verbo do winetricks instalado como o usuário, sem senha,
//! num prefixo do Wine só do Vega (`~/.local/share/wineprefixes/lyra-windows`),
//! para não mexer no `~/.wine` de quem já usa o Wine. O winetricks baixa o
//! instalador do fabricante, confere o checksum e roda sem perguntas; o Wine
//! cria os atalhos no menu. Só os verbos de `VERBS` são aceitos: a interface
//! nunca passa texto livre para o winetricks.
//!
//! Os instaladores são quase todos de 32 bits, e o kernel do Leap 16 vem com a
//! emulação de 32 bits desligada (CONFIG_IA32_EMULATION_DEFAULT_DISABLED): sem
//! `ia32_emulation=1` no boot, o Wine (WoW64) cria o processo e ele morre ao
//! entrar em 32 bits. `windows_ia32` diz se o kernel em execução executa
//! código de 32 bits, para a interface pedir o parâmetro antes de instalar.
//! Um prefixo criado enquanto o 32 bits estava desligado fica sem
//! `syswow64` e não se recupera com `wineboot -u`: `set_aside_broken_prefix`
//! o renomeia (sem apagar) para o winetricks criar um novo.
//!
//! O catálogo do winetricks é pequeno e antigo; os apps de `RECIPES` são
//! receitas do próprio Vega no mesmo espírito: URL oficial do instalador,
//! SHA-256 fixado e os parâmetros de instalação silenciosa do fabricante.
//! O que elas instalam fica registrado em `lyra-vega.log`, no prefixo.
//! `windows_run_file` roda um `.exe`/`.msi` escolhido pelo usuário no
//! mesmo prefixo.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use gettextrs::gettext;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use super::flatpak_session::follow_then;
use super::software::TransactionEvent;

/// Verbos que a vitrine oferece (todos com `media="download"`).
pub const VERBS: &[&str] = &[
    "7zip",
    "winamp",
    "autohotkey",
    "mt4",
    "kindle",
    "adobe_diged4",
    "kobo",
    "ubisoftconnect",
    "mspaint",
    "sketchup",
];

/// Como uma receita instala o arquivo baixado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Instalador rodado pelo Wine com estes parâmetros silenciosos.
    Installer(&'static [&'static str]),
    /// Programa portátil: copiado para `C:\Program Files\<dir>` e ganha um
    /// atalho no menu com o nome `name`.
    Portable {
        dir: &'static str,
        name: &'static str,
    },
}

/// Receita do Vega: instalador oficial com checksum fixado.
#[derive(Debug, Clone, Copy)]
pub struct Recipe {
    pub id: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub file: &'static str,
    pub kind: Kind,
}

/// Parâmetros silenciosos dos instaladores Inno Setup.
const INNO: &[&str] = &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"];

/// Para atualizar uma receita: troque URL e arquivo, baixe e confira o
/// SHA-256 com o publicado pelo projeto quando houver (o do Notepad++ vem em
/// `npp.<versão>.checksums.sha256`, assinado). O `npp` aqui substitui o verbo
/// do winetricks, que ainda instala a 7.7.1.
pub const RECIPES: &[Recipe] = &[
    // O verbo winrar do winetricks aponta para a 6.11, que a RARLAB tirou do
    // ar (o endereço devolve uma página HTML).
    Recipe {
        id: "winrar",
        url: "https://www.rarlab.com/rar/winrar-x64-723br.exe",
        sha256: "6ebe807d40f179c12b40cde6a9daef03e7cdb06b904b6732ade9cec9e6d4d56b",
        file: "winrar-x64-723br.exe",
        kind: Kind::Installer(&["/S"]),
    },
    Recipe {
        id: "npp",
        url: "https://github.com/notepad-plus-plus/notepad-plus-plus/releases/download/v8.9.8.1/npp.8.9.8.1.Installer.x64.exe",
        sha256: "26f3bcced788fadbac4e7e577b430ea2ce6060cf3bcad993ef4612ecd7d743da",
        file: "npp.8.9.8.1.Installer.x64.exe",
        kind: Kind::Installer(&["/S"]),
    },
    Recipe {
        id: "heidisql",
        url: "https://github.com/HeidiSQL/HeidiSQL/releases/download/v12.22/HeidiSQL_12.22.0.7382_Setup.exe",
        sha256: "468055814dcedc057821f2692d28cfdde181332265240538b9acb320cae49904",
        file: "HeidiSQL_12.22.0.7382_Setup.exe",
        kind: Kind::Installer(INNO),
    },
    Recipe {
        id: "winmerge",
        url: "https://github.com/WinMerge/winmerge/releases/download/v2.16.60/WinMerge-2.16.60-x64-Setup.exe",
        sha256: "ed01097f60b63b3e7d58b3036064c40f05504a47c23c231a1081142295815f7f",
        file: "WinMerge-2.16.60-x64-Setup.exe",
        kind: Kind::Installer(INNO),
    },
    Recipe {
        id: "sumatrapdf",
        url: "https://www.sumatrapdfreader.org/dl/rel/3.6.1/SumatraPDF-3.6.1-64-install.exe",
        sha256: "1eee71cccd2ea6e94d5bcea54ee2f759844da3e1a0ee2f6045035b1d17b94381",
        file: "SumatraPDF-3.6.1-64-install.exe",
        kind: Kind::Installer(&["-s", "-all-users"]),
    },
    Recipe {
        id: "winbox",
        url: "https://download.mikrotik.com/routeros/winbox/3.41/winbox64.exe",
        sha256: "8bc3ecf1f35952600ecb1a380c38c88e9d63c081a32204fd094d588230070bf6",
        file: "winbox64.exe",
        kind: Kind::Portable {
            dir: "MikroTik Winbox",
            name: "Winbox",
        },
    },
];

pub fn recipe(id: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|recipe| recipe.id == id)
}

/// Id oferecido pela vitrine (verbo do winetricks ou receita do Vega)?
pub fn offered(id: &str) -> bool {
    VERBS.contains(&id) || recipe(id).is_some()
}

/// Verbos cujo instalador o winetricks conduz com um script do AutoHotkey,
/// que espera os títulos das janelas em inglês. Com o Wine em português o
/// script não acha a janela e a instalação fica parada para sempre (visto no
/// Winamp); para eles o winetricks roda com o locale C, que o Wine trata
/// como en-US.
const AHK_VERBS: &[&str] = &["winamp", "mt4", "sketchup"];

/// Linha do `w_askpermission` do winetricks que, no modo `--unattended` sem
/// interface, responde "sim" a qualquer pergunta, inclusive "SHA256
/// mismatch! Continue anyway?": um download trocado ou adulterado seria
/// executado (aconteceu com o WinRAR, cujo endereço passou a devolver HTML).
const WINETRICKS_AUTO_YES: &str = "w_info \"Unattended mode, not prompting for confirmation\"";

/// Cópia do winetricks em que essa resposta automática vira "não": o
/// winetricks cancela com "Operação cancelada" em vez de rodar o arquivo.
/// Sem a linha exata (outra versão do winetricks), devolve `None` e a
/// instalação é recusada: melhor não instalar do que instalar sem checksum.
pub fn guarded_winetricks(script: &str) -> Option<String> {
    (script.matches(WINETRICKS_AUTO_YES).count() == 1).then(|| {
        script.replacen(
            WINETRICKS_AUTO_YES,
            &format!("{WINETRICKS_AUTO_YES}; false"),
            1,
        )
    })
}

/// Grava a cópia protegida em `$XDG_RUNTIME_DIR/lyra-vega/winetricks`.
fn guarded_winetricks_path() -> Result<PathBuf, String> {
    let original =
        std::fs::read_to_string("/usr/bin/winetricks").map_err(|error| error.to_string())?;
    let guarded = guarded_winetricks(&original).ok_or_else(|| {
        gettext("Esta versão do winetricks não pôde ser preparada para conferir os downloads; nada foi instalado.")
    })?;
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(std::env::temp_dir)
        .join("lyra-vega");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join("winetricks");
    std::fs::write(&path, guarded).map_err(|error| error.to_string())?;
    Ok(path)
}

/// Ids próprios, acima dos do vegad e dos do Flatpak na sessão.
static NEXT_ID: AtomicU32 = AtomicU32::new(0xC000_0000);

fn prefix() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(data.join("wineprefixes/lyra-windows"))
}

/// Valor de `ia32_emulation=` na linha de comando do kernel, se houver.
pub fn ia32_from_cmdline(cmdline: &str) -> Option<bool> {
    cmdline
        .split_whitespace()
        .rev()
        .find_map(|arg| arg.strip_prefix("ia32_emulation="))
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "y" | "yes" | "on" | "true" => Some(true),
            "0" | "n" | "no" | "off" | "false" => Some(false),
            _ => None,
        })
}

/// Padrão de compilação do kernel: sem a emulação, ou com ela desligada por
/// padrão, não há 32 bits; sem a config legível, presume que há.
pub fn ia32_from_config(config: &str) -> bool {
    let has = |line: &str| config.lines().any(|l| l.trim() == line);
    if has("CONFIG_IA32_EMULATION_DEFAULT_DISABLED=y") {
        return false;
    }
    !config.contains("CONFIG_IA32_EMULATION") || has("CONFIG_IA32_EMULATION=y")
}

/// O kernel em execução roda programas de 32 bits?
#[tauri::command]
pub fn windows_ia32() -> bool {
    let cmdline = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();
    if let Some(value) = ia32_from_cmdline(&cmdline) {
        return value;
    }
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    std::fs::read_to_string(format!("/boot/config-{}", release.trim()))
        .map(|config| ia32_from_config(&config))
        .unwrap_or(true)
}

/// Prefixo inutilizável: existe (tem `system.reg`) mas a `syswow64` está
/// vazia ou ausente, como fica quando o Wine o criou sem suporte a 32 bits.
pub fn prefix_is_broken(prefix: &Path) -> bool {
    prefix.join("system.reg").is_file()
        && std::fs::read_dir(prefix.join("drive_c/windows/syswow64"))
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true)
}

/// Renomeia um prefixo quebrado para `<nome>.quebrado-<segundos>` e devolve o
/// novo caminho. Só age com o 32 bits ligado: sem ele, um prefixo novo
/// nasceria quebrado do mesmo jeito.
pub fn set_aside_broken_prefix(prefix: &Path, ia32: bool) -> std::io::Result<Option<PathBuf>> {
    if !ia32 || !prefix_is_broken(prefix) {
        return Ok(None);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let name = prefix
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("lyra-windows");
    let aside = prefix.with_file_name(format!("{name}.quebrado-{stamp}"));
    std::fs::rename(prefix, &aside)?;
    Ok(Some(aside))
}

/// O Wine grava os atalhos em `~/.local/share/applications/wine/Programs`, e
/// o GNOME só vigia a pasta `applications` em si: sem uma mudança nela, o
/// app instalado não aparece no menu até o próximo login. O
/// update-desktop-database reescreve o `mimeinfo.cache` dali e dispara a
/// releitura.
fn refresh_menu(data: &Path) {
    let mut command = Command::new("update-desktop-database");
    command.arg(data.join("applications"));
    crate::webkit_env::clean_command(&mut command);
    let _ = command.status();
}

/// `~/.local/share` (ou `$XDG_DATA_HOME`).
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

/// Marca, dentro do prefixo, de que as associações de arquivo já foram
/// desligadas nele.
const ASSOCIATIONS_OFF: &str = ".lyra-vega-associations-off";

/// Desliga as associações de arquivo do Wine no prefixo (uma vez): sem isso,
/// cada programa instalado se registra como leitor dos tipos de arquivo que
/// conhece (o Winamp criou 86 associações, de mp3 a mkv, e 29 tipos MIME em
/// `~/.local/share/mime`). Os atalhos do menu continuam sendo criados.
fn prepare_prefix(prefix: &Path) {
    if prefix.join(ASSOCIATIONS_OFF).exists() {
        return;
    }
    let wine = |program: &str, args: &[&str]| {
        let mut command = Command::new(program);
        command
            .args(args)
            .env("WINEPREFIX", prefix)
            .env("WINEDEBUG", "-all");
        crate::webkit_env::clean_command(&mut command);
        command.status().is_ok_and(|status| status.success())
    };
    if !prefix.join("system.reg").is_file() && !wine("wineboot", &["-i"]) {
        return;
    }
    let off = wine(
        "wine",
        &[
            "reg",
            "add",
            "HKEY_CURRENT_USER\\Software\\Wine\\FileOpenAssociations",
            "/v",
            "Enable",
            "/t",
            "REG_SZ",
            "/d",
            "N",
            "/f",
        ],
    );
    if off {
        let _ = std::fs::write(prefix.join(ASSOCIATIONS_OFF), "");
    }
}

/// Atalho acessório que não precisa ficar no menu: desinstalador, "o que há
/// de novo", modo de segurança, leia-me, licença, site do fabricante, guia
/// do usuário, documentação e ajuda.
pub fn is_accessory(name: &str) -> bool {
    const WORDS: &[&str] = &[
        "desinstal",
        "uninstall",
        "o que há de novo",
        "what's new",
        "whats new",
        "modo de segurança",
        "safe mode",
        "leia-me",
        "leiame",
        "readme",
        "read me",
        "release notes",
        "notas de versão",
        "changelog",
        "license",
        "licença",
        "website",
        "web site",
        "on the web",
        "guia do usu",
        "user guide",
        "user's guide",
        "manual do usu",
        "user manual",
        "documenta",
        "ajuda",
        "help",
    ];
    let name = name.to_lowercase();
    WORDS.iter().any(|word| name.contains(word))
}

fn desktop_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            desktop_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "desktop") {
            found.push(path);
        }
    }
}

/// Arruma o menu depois de instalar no prefixo: remove as associações de
/// arquivo que o Wine criou para ele (`wine-extension-*`/`wine-protocol-*`
/// e os tipos `x-wine-extension-*` que só elas usavam) e esconde
/// (`NoDisplay=true`) os atalhos acessórios. Só toca em arquivos cujo `Exec`
/// aponta para este prefixo. Devolve se os tipos MIME mudaram.
pub fn tidy_menu(data: &Path, prefix: &Path) -> bool {
    let apps = data.join("applications");
    let tag = format!("WINEPREFIX={}\"", prefix.display());
    let mut files = Vec::new();
    desktop_files(&apps, &mut files);
    let mut removed_ext = Vec::new();
    let mut kept_ext = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let ext = file
            .strip_prefix("wine-extension-")
            .and_then(|rest| rest.strip_suffix(".desktop"));
        let association = ext.is_some() || file.starts_with("wine-protocol-");
        if association && path.parent() == Some(apps.as_path()) {
            if text.contains(&tag) {
                if std::fs::remove_file(path).is_ok()
                    && let Some(ext) = ext
                {
                    removed_ext.push(ext.to_owned());
                }
            } else if let Some(ext) = ext {
                kept_ext.push(ext.to_owned());
            }
            continue;
        }
        let name = text
            .lines()
            .find_map(|line| line.strip_prefix("Name="))
            .unwrap_or("");
        if text.contains(&tag)
            && is_accessory(name)
            && !text.lines().any(|line| line.trim() == "NoDisplay=true")
        {
            let hidden = text.replacen("[Desktop Entry]\n", "[Desktop Entry]\nNoDisplay=true\n", 1);
            let _ = std::fs::write(path, hidden);
        }
    }
    let mut mime_changed = false;
    for ext in removed_ext.iter().filter(|ext| !kept_ext.contains(ext)) {
        let xml = data.join(format!("mime/packages/x-wine-extension-{ext}.xml"));
        if std::fs::remove_file(xml).is_ok() {
            mime_changed = true;
        }
    }
    mime_changed
}

/// Fim de toda instalação no prefixo: arruma o menu e faz o GNOME reler.
fn after_install(prefix: &Path) {
    let Some(data) = data_dir() else {
        return;
    };
    if tidy_menu(&data, prefix) {
        let mut command = Command::new("update-mime-database");
        command.arg(data.join("mime"));
        crate::webkit_env::clean_command(&mut command);
        let _ = command.status();
    }
    refresh_menu(&data);
}

/// Ids registrados no `winetricks.log` e no `lyra-vega.log` do prefixo (um
/// por linha), só os que a vitrine oferece.
pub fn installed_from_log(log: &str) -> Vec<String> {
    let mut verbs: Vec<String> = log
        .lines()
        .map(str::trim)
        .filter(|verb| offered(verb))
        .map(str::to_owned)
        .collect();
    verbs.sort();
    verbs.dedup();
    verbs
}

#[tauri::command]
pub fn windows_installed() -> Vec<String> {
    let Some(prefix) = prefix() else {
        return Vec::new();
    };
    let log = ["winetricks.log", "lyra-vega.log"]
        .iter()
        .filter_map(|name| std::fs::read_to_string(prefix.join(name)).ok())
        .collect::<Vec<_>>()
        .join("\n");
    installed_from_log(&log)
}

fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .map(|cache| cache.join("lyra-vega/windows"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Baixa (ou reaproveita do cache) o instalador e confere o SHA-256. Só
/// HTTPS, e nada é gravado com o nome final antes de o checksum bater.
fn fetch_verified(recipe: &Recipe) -> Result<PathBuf, String> {
    let dir = cache_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let target = dir.join(recipe.file);
    if let Ok(bytes) = std::fs::read(&target)
        && sha256_hex(&bytes) == recipe.sha256
    {
        return Ok(target);
    }
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|error| error.to_string())?;
    let bytes = client
        .get(recipe.url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.bytes())
        .map_err(|error| error.to_string())?;
    if sha256_hex(&bytes) != recipe.sha256 {
        return Err(gettext(
            "O instalador baixado não confere com o checksum esperado; nada foi instalado.",
        ));
    }
    let partial = dir.join(format!("{}.part", recipe.file));
    std::fs::write(&partial, &bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&partial, &target).map_err(|error| error.to_string())?;
    Ok(target)
}

/// Atalho de menu para `C:\<partes>` no prefixo. Dentro de aspas no `Exec`,
/// uma barra invertida literal vira quatro no arquivo (escape de string e
/// depois de aspas), como o próprio Wine grava.
pub fn windows_desktop_entry(prefix: &Path, parts: &[&str], name: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\nExec=env \"WINEPREFIX={prefix}\" wine \"C:\\\\\\\\{path}\"\nIcon=wine\nStartupNotify=true\n",
        comment = gettext("Programa do Windows"),
        prefix = prefix.display(),
        path = parts.join("\\\\\\\\"),
    )
}

/// Grava `lyra-windows-<id>.desktop` em `~/.local/share/applications`.
fn write_shortcut(id: &str, entry: &str) -> Result<(), String> {
    let data = data_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let applications = data.join("applications");
    std::fs::create_dir_all(&applications).map_err(|error| error.to_string())?;
    std::fs::write(
        applications.join(format!("lyra-windows-{id}.desktop")),
        entry,
    )
    .map_err(|error| error.to_string())
}

/// Verbos do winetricks que só copiam o programa, sem criar atalho no menu
/// (o `mspaint` põe o Paint do XP em `C:\windows`): o Vega cria o atalho.
const SHORTCUTS: &[(&str, &[&str], &str)] = &[("mspaint", &["windows", "mspaint.exe"], "Paint")];

fn install_portable(
    prefix: &Path,
    recipe: &Recipe,
    dir: &str,
    name: &str,
    file: &Path,
) -> Result<(), String> {
    if !prefix.join("system.reg").is_file() {
        let mut boot = Command::new("wineboot");
        boot.arg("-i")
            .env("WINEPREFIX", prefix)
            .env("WINEDEBUG", "-all");
        crate::webkit_env::clean_command(&mut boot);
        boot.status().map_err(|error| error.to_string())?;
    }
    let dest = prefix.join("drive_c/Program Files").join(dir);
    std::fs::create_dir_all(&dest).map_err(|error| error.to_string())?;
    std::fs::copy(file, dest.join(recipe.file)).map_err(|error| error.to_string())?;
    write_shortcut(
        recipe.id,
        &windows_desktop_entry(prefix, &["Program Files", dir, recipe.file], name),
    )
}

fn record_installed(prefix: &Path, id: &str) {
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(prefix.join("lyra-vega.log"))
    {
        let _ = writeln!(log, "{id}");
    }
}

fn run_recipe(app: &AppHandle, tx: u32, recipe: &Recipe, prefix: &Path) {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    let failed = gettext("A instalação do aplicativo do Windows falhou.");
    emit(TransactionEvent::Progress {
        id: tx,
        percent: 5,
        message: gettext("Baixando {file}…").replace("{file}", recipe.file),
    });
    let file = match fetch_verified(recipe) {
        Ok(file) => file,
        Err(message) => {
            emit(TransactionEvent::Finished {
                id: tx,
                success: false,
                message,
            });
            return;
        }
    };
    emit(TransactionEvent::Progress {
        id: tx,
        percent: 30,
        message: gettext("Instalando {file}…").replace("{file}", recipe.file),
    });
    match recipe.kind {
        Kind::Installer(args) => {
            let mut command = Command::new("wine");
            command
                .arg(&file)
                .args(args)
                .env("WINEPREFIX", prefix)
                .env("WINEDEBUG", "-all");
            crate::webkit_env::clean_command(&mut command);
            follow_then(app, tx, command, "wine", failed, |success| {
                if success {
                    record_installed(prefix, recipe.id);
                }
                after_install(prefix);
            });
        }
        Kind::Portable { dir, name } => {
            let result = install_portable(prefix, recipe, dir, name, &file);
            let success = result.is_ok();
            if success {
                record_installed(prefix, recipe.id);
            }
            after_install(prefix);
            emit(TransactionEvent::Finished {
                id: tx,
                success,
                message: result.err().unwrap_or_else(|| gettext("Concluído")),
            });
        }
    }
}

/// Prefixo pronto para uso: renomeia o quebrado antes de instalar.
fn ready_prefix() -> Result<PathBuf, String> {
    let prefix = prefix().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    set_aside_broken_prefix(&prefix, windows_ia32()).map_err(|error| {
        format!(
            "{}: {error}",
            gettext("Não foi possível recriar o ambiente do Wine")
        )
    })?;
    Ok(prefix)
}

/// Começa a instalação e devolve o id da tarefa na hora.
#[tauri::command]
pub fn windows_install(app: AppHandle, verb: String) -> Result<u32, String> {
    if !offered(&verb) {
        return Err(gettext("Aplicativo do Windows desconhecido."));
    }
    let prefix = ready_prefix()?;
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Some(recipe) = recipe(&verb) {
        std::thread::spawn(move || {
            prepare_prefix(&prefix);
            run_recipe(&app, tx, recipe, &prefix);
        });
        return Ok(tx);
    }
    std::thread::spawn(move || {
        prepare_prefix(&prefix);
        let winetricks = match guarded_winetricks_path() {
            Ok(path) => path,
            Err(message) => {
                let _ = app.emit(
                    "software-transaction",
                    TransactionEvent::Finished {
                        id: tx,
                        success: false,
                        message,
                    },
                );
                return;
            }
        };
        let mut command = Command::new("/bin/sh");
        command
            .arg(&winetricks)
            .args(["--unattended", &verb])
            .env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "-all");
        if AHK_VERBS.contains(&verb.as_str()) {
            command.env("LC_ALL", "C.UTF-8").env("LANG", "C.UTF-8");
        }
        crate::webkit_env::clean_command(&mut command);
        follow_then(
            &app,
            tx,
            command,
            "winetricks",
            gettext("A instalação do aplicativo do Windows falhou."),
            |success| {
                if success
                    && let Some((_, parts, name)) = SHORTCUTS.iter().find(|(id, ..)| *id == verb)
                {
                    let _ = write_shortcut(&verb, &windows_desktop_entry(&prefix, parts, name));
                }
                after_install(&prefix);
            },
        );
    });
    Ok(tx)
}

/// Como remover um app da vitrine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// Desinstalador registrado no Wine, achado pelo começo do DisplayName,
    /// rodado com estes parâmetros silenciosos (`.msi`: `msiexec /x /qn`).
    Uninstaller {
        display: &'static str,
        args: &'static [&'static str],
    },
    /// Programa só copiado para o prefixo: apaga estes caminhos.
    Files(&'static [&'static str]),
}

const NSIS: &[&str] = &["/S"];

pub const REMOVALS: &[(&str, Removal)] = &[
    (
        "7zip",
        Removal::Uninstaller {
            display: "7-Zip",
            args: NSIS,
        },
    ),
    (
        "winrar",
        Removal::Uninstaller {
            display: "WinRAR",
            args: NSIS,
        },
    ),
    (
        "npp",
        Removal::Uninstaller {
            display: "Notepad++",
            args: NSIS,
        },
    ),
    (
        "winmerge",
        Removal::Uninstaller {
            display: "WinMerge",
            args: INNO,
        },
    ),
    (
        "autohotkey",
        Removal::Uninstaller {
            display: "AutoHotkey",
            args: &[],
        },
    ),
    (
        "sumatrapdf",
        Removal::Uninstaller {
            display: "SumatraPDF",
            args: &["-s"],
        },
    ),
    (
        "kindle",
        Removal::Uninstaller {
            display: "Kindle",
            args: NSIS,
        },
    ),
    (
        "adobe_diged4",
        Removal::Uninstaller {
            display: "Adobe Digital Editions",
            args: NSIS,
        },
    ),
    (
        "kobo",
        Removal::Uninstaller {
            display: "Kobo",
            args: NSIS,
        },
    ),
    ("mspaint", Removal::Files(&["drive_c/windows/mspaint.exe"])),
    (
        "sketchup",
        Removal::Uninstaller {
            display: "Google SketchUp",
            args: &[],
        },
    ),
    (
        "winbox",
        Removal::Files(&["drive_c/Program Files/MikroTik Winbox"]),
    ),
    (
        "heidisql",
        Removal::Uninstaller {
            display: "HeidiSQL",
            args: INNO,
        },
    ),
    (
        "winamp",
        Removal::Uninstaller {
            display: "Winamp",
            args: NSIS,
        },
    ),
    (
        "ubisoftconnect",
        Removal::Uninstaller {
            display: "Ubisoft Connect",
            args: NSIS,
        },
    ),
    (
        "mt4",
        Removal::Uninstaller {
            display: "MetaTrader",
            args: NSIS,
        },
    ),
];

pub fn removal(id: &str) -> Option<Removal> {
    REMOVALS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, removal)| *removal)
}

/// (DisplayName, UninstallString) da saída de `wine reg query … /s`.
pub fn uninstall_entries(text: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut name: Option<String> = None;
    let mut command: Option<String> = None;
    let mut flush = |name: &mut Option<String>, command: &mut Option<String>| {
        if let (Some(n), Some(c)) = (name.take(), command.take()) {
            entries.push((n, c));
        }
    };
    for line in text.lines() {
        if line.starts_with("HKEY_") {
            flush(&mut name, &mut command);
            continue;
        }
        let mut parts = line.trim().splitn(3, "    ");
        let (Some(key), Some(_kind), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        match key {
            "DisplayName" => name = Some(value.trim().to_owned()),
            "UninstallString" => command = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    flush(&mut name, &mut command);
    entries
}

/// Separa o programa dos argumentos de um UninstallString (com ou sem
/// aspas; sem aspas, o caminho vai até o `.exe`, que pode ter espaços).
pub fn split_uninstall(command: &str) -> (String, Vec<String>) {
    let command = command.trim();
    let (program, rest) = if let Some(quoted) = command.strip_prefix('"') {
        match quoted.find('"') {
            Some(end) => (quoted[..end].to_owned(), &quoted[end + 1..]),
            None => (quoted.to_owned(), ""),
        }
    } else {
        let lower = command.to_ascii_lowercase();
        match lower.find(".exe") {
            Some(at) => (command[..at + 4].to_owned(), &command[at + 4..]),
            None => (command.to_owned(), ""),
        }
    };
    (
        program,
        rest.split_whitespace().map(str::to_owned).collect(),
    )
}

/// Comando final: msiexec troca /I por /X e ganha /qn; os outros recebem os
/// parâmetros silenciosos da receita.
pub fn uninstall_command(command: &str, silent: &[&str]) -> (String, Vec<String>) {
    let (program, mut args) = split_uninstall(command);
    if program.to_ascii_lowercase().ends_with("msiexec.exe") {
        for arg in &mut args {
            if let Some(code) = arg.strip_prefix("/I").or_else(|| arg.strip_prefix("/i")) {
                *arg = format!("/X{code}");
            }
        }
        args.push("/qn".into());
    } else {
        args.extend(silent.iter().map(|arg| (*arg).to_owned()));
    }
    (program, args)
}

fn wine_reg_uninstall(prefix: &Path) -> String {
    [
        "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "HKLM\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ]
    .iter()
    .filter_map(|key| {
        let mut command = Command::new("wine");
        command
            .args(["reg", "query", key, "/s"])
            .env("WINEPREFIX", prefix)
            .env("WINEDEBUG", "-all");
        crate::webkit_env::clean_command(&mut command);
        command
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// Tira `id` dos registros de instalados (`winetricks.log`, `lyra-vega.log`).
fn forget_installed(prefix: &Path, id: &str) {
    for name in ["winetricks.log", "lyra-vega.log"] {
        let path = prefix.join(name);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let kept: String = text
                .lines()
                .filter(|line| line.trim() != id)
                .map(|line| format!("{line}\n"))
                .collect();
            let _ = std::fs::write(&path, kept);
        }
    }
}

/// Caminho Unix de `C:\\…` no prefixo, a partir do `Exec` de um atalho
/// (barra invertida escrita como `\\\\` no arquivo).
pub fn exec_target(exec: &str, prefix: &Path) -> Option<PathBuf> {
    let start = exec.find("wine \"C:")? + "wine \"C:".len();
    let rest = &exec[start..];
    let windows = &rest[..rest.find('"')?];
    let relative = windows.replace("\\\\", "/").replace('\\', "/");
    Some(
        prefix
            .join("drive_c")
            .join(relative.trim_start_matches('/')),
    )
}

/// Apaga os atalhos deste prefixo que apontam para algo que não existe mais
/// (depois de desinstalar) e as pastas de menu que ficaram vazias.
pub fn remove_stale_shortcuts(data: &Path, prefix: &Path) {
    let apps = data.join("applications");
    let tag = format!("WINEPREFIX={}\"", prefix.display());
    let mut files = Vec::new();
    desktop_files(&apps, &mut files);
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains(&tag) {
            continue;
        }
        let exec = text
            .lines()
            .find_map(|line| line.strip_prefix("Exec="))
            .unwrap_or("");
        if let Some(target) = exec_target(exec, prefix)
            && !target.exists()
        {
            let _ = std::fs::remove_file(&path);
        }
    }
    let programs = apps.join("wine/Programs");
    if let Ok(entries) = std::fs::read_dir(&programs) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_dir(entry.path());
        }
    }
}

/// Depois de remover: esquece o app, apaga atalhos órfãos e relê o menu.
fn after_removal(prefix: &Path, id: &str, success: bool) {
    if success {
        forget_installed(prefix, id);
    }
    if let Some(data) = data_dir() {
        remove_stale_shortcuts(&data, prefix);
    }
    after_install(prefix);
}

fn run_removal(app: &AppHandle, tx: u32, id: &str, removal: Removal, prefix: &Path) {
    let emit = |event: TransactionEvent| {
        let _ = app.emit("software-transaction", event);
    };
    match removal {
        Removal::Files(paths) => {
            let mut result = Ok(());
            for relative in paths {
                let path = prefix.join(relative);
                let outcome = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                if let Err(error) = outcome
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    result = Err(error.to_string());
                }
            }
            if let Some(data) = data_dir() {
                let _ = std::fs::remove_file(
                    data.join(format!("applications/lyra-windows-{id}.desktop")),
                );
            }
            after_removal(prefix, id, result.is_ok());
            emit(TransactionEvent::Finished {
                id: tx,
                success: result.is_ok(),
                message: result.err().unwrap_or_else(|| gettext("Concluído")),
            });
        }
        Removal::Uninstaller { display, args } => {
            let entries = uninstall_entries(&wine_reg_uninstall(prefix));
            let Some((_, command)) = entries.iter().find(|(name, _)| name.starts_with(display))
            else {
                // Já não está registrado no Wine: só esquece.
                after_removal(prefix, id, true);
                emit(TransactionEvent::Finished {
                    id: tx,
                    success: true,
                    message: gettext("Concluído"),
                });
                return;
            };
            let (program, arguments) = uninstall_command(command, args);
            let mut wine = Command::new("wine");
            wine.arg(&program)
                .args(&arguments)
                .env("WINEPREFIX", prefix)
                .env("WINEDEBUG", "-all");
            crate::webkit_env::clean_command(&mut wine);
            follow_then(
                app,
                tx,
                wine,
                "wine",
                gettext("A remoção do aplicativo do Windows falhou."),
                |success| after_removal(prefix, id, success),
            );
        }
    }
}

/// Remove um app da vitrine e devolve o id da tarefa na hora.
#[tauri::command]
pub fn windows_remove(app: AppHandle, id: String) -> Result<u32, String> {
    let Some(removal) = removal(&id) else {
        return Err(gettext("Aplicativo do Windows desconhecido."));
    };
    let prefix = prefix().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        run_removal(&app, tx, &id, removal, &prefix);
    });
    Ok(tx)
}

/// Programa do Windows escolhido pelo usuário (`.exe` ou `.msi`)?
pub fn runnable_file(path: &Path) -> bool {
    path.is_absolute()
        && path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("msi"))
}

/// Roda um `.exe`/`.msi` escolhido pelo usuário no prefixo do Vega, como o
/// usuário, e acompanha até o programa fechar. `start /wait /unix` abre o
/// `.msi` pelo msiexec e espera o instalador terminar.
#[tauri::command]
pub fn windows_run_file(app: AppHandle, path: String) -> Result<u32, String> {
    let file = PathBuf::from(&path);
    if !runnable_file(&file) {
        return Err(gettext("Escolha um arquivo .exe ou .msi."));
    }
    let prefix = ready_prefix()?;
    let tx = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        prepare_prefix(&prefix);
        let mut command = Command::new("wine");
        command
            .args(["start", "/wait", "/unix"])
            .arg(&file)
            .env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "-all");
        if let Some(parent) = file.parent() {
            command.current_dir(parent);
        }
        crate::webkit_env::clean_command(&mut command);
        follow_then(
            &app,
            tx,
            command,
            "wine",
            gettext("O programa do Windows terminou com erro."),
            |_| after_install(&prefix),
        );
    });
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_keeps_only_offered_verbs() {
        let log = "vcrun2019\nnpp\n7zip\nnpp\n  winrar \ncorefonts\n";
        assert_eq!(installed_from_log(log), ["7zip", "npp", "winrar"]);
    }

    #[test]
    fn ia32_follows_the_kernel_parameter_then_the_build_default() {
        assert_eq!(ia32_from_cmdline("quiet splash"), None);
        assert_eq!(
            ia32_from_cmdline("quiet ia32_emulation=1 splash"),
            Some(true)
        );
        assert_eq!(
            ia32_from_cmdline("ia32_emulation=on ia32_emulation=0"),
            Some(false)
        );
        assert_eq!(ia32_from_cmdline("ia32_emulation=talvez"), None);
        let leap = "CONFIG_IA32_EMULATION=y\nCONFIG_IA32_EMULATION_DEFAULT_DISABLED=y\n";
        assert!(!ia32_from_config(leap));
        assert!(ia32_from_config(
            "CONFIG_IA32_EMULATION=y\n# CONFIG_IA32_EMULATION_DEFAULT_DISABLED is not set\n"
        ));
        assert!(!ia32_from_config("# CONFIG_IA32_EMULATION is not set\n"));
        assert!(ia32_from_config(""));
    }

    #[test]
    fn broken_prefix_is_set_aside_only_with_ia32() {
        let root = std::env::temp_dir().join(format!("vega-wine-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let prefix = root.join("lyra-windows");
        std::fs::create_dir_all(prefix.join("drive_c/windows/syswow64")).unwrap();
        std::fs::write(prefix.join("system.reg"), "").unwrap();
        assert!(prefix_is_broken(&prefix));
        assert_eq!(set_aside_broken_prefix(&prefix, false).unwrap(), None);
        assert!(prefix.exists());

        let aside = set_aside_broken_prefix(&prefix, true).unwrap().unwrap();
        assert!(!prefix.exists() && aside.join("system.reg").is_file());
        assert!(
            aside
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("lyra-windows.quebrado-")
        );

        // Prefixo saudável ou ainda inexistente: nada muda.
        std::fs::create_dir_all(prefix.join("drive_c/windows/syswow64")).unwrap();
        std::fs::write(prefix.join("system.reg"), "").unwrap();
        std::fs::write(prefix.join("drive_c/windows/syswow64/ntdll.dll"), "").unwrap();
        assert!(!prefix_is_broken(&prefix));
        assert!(!prefix_is_broken(&root.join("ausente")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn recipes_are_pinned_https_downloads() {
        for recipe in RECIPES {
            assert!(recipe.url.starts_with("https://"), "{}", recipe.id);
            assert!(recipe.url.ends_with(recipe.file), "{}", recipe.id);
            assert_eq!(recipe.sha256.len(), 64, "{}", recipe.id);
            assert!(
                recipe
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{}",
                recipe.id
            );
            assert!(!recipe.file.contains(['/', '\\']), "{}", recipe.id);
        }
        let mut ids: Vec<&str> = RECIPES
            .iter()
            .map(|recipe| recipe.id)
            .chain(VERBS.iter().copied())
            .collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(
            before,
            ids.len(),
            "um id não pode ser verbo e receita ao mesmo tempo"
        );
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn installed_log_accepts_recipes_too() {
        assert_eq!(
            installed_from_log("npp\nwinbox\nnada\n7zip\n"),
            ["7zip", "npp", "winbox"]
        );
    }

    #[test]
    fn portable_entry_points_into_the_prefix() {
        let entry = windows_desktop_entry(
            Path::new("/home/u/.local/share/wineprefixes/lyra-windows"),
            &["Program Files", "MikroTik Winbox", "winbox64.exe"],
            "Winbox",
        );
        assert!(entry.contains("Name=Winbox\n"));
        assert!(entry.contains("Exec=env \"WINEPREFIX=/home/u/.local/share/wineprefixes/lyra-windows\" wine \"C:\\\\\\\\Program Files\\\\\\\\MikroTik Winbox\\\\\\\\winbox64.exe\"\n"));
    }

    #[test]
    fn shortcut_ids_are_offered_verbs() {
        let paint = windows_desktop_entry(Path::new("/p"), &["windows", "mspaint.exe"], "Paint");
        assert!(paint.contains("wine \"C:\\\\\\\\windows\\\\\\\\mspaint.exe\"\n"));
        for (id, ..) in SHORTCUTS {
            assert!(offered(id), "{id}");
        }
    }

    #[test]
    fn accessories_are_recognized() {
        for name in [
            "Desinstalar Winamp",
            "Uninstall Notepad++",
            "O Que há de Novo",
            "Winamp (Modo de Segurança)",
            "Leia-me",
            "README",
            "License",
            "Guia do Usuário",
            "WinMerge Help",
            "Documentação",
        ] {
            assert!(is_accessory(name), "{name}");
        }
        for name in ["Winamp", "Notepad++", "HeidiSQL", "Paint"] {
            assert!(!is_accessory(name), "{name}");
        }
    }

    #[test]
    fn tidy_menu_only_touches_this_prefix() {
        let root = std::env::temp_dir().join(format!("vega-tidy-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("share");
        let apps = data.join("applications");
        std::fs::create_dir_all(apps.join("wine/Programs/Winamp")).unwrap();
        std::fs::create_dir_all(data.join("mime/packages")).unwrap();
        let ours = Path::new("/home/u/.local/share/wineprefixes/lyra-windows");
        let exec = |prefix: &str| format!("Exec=env \"WINEPREFIX={prefix}\" wine x\n");
        let entry = |name: &str, prefix: &str| {
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\n{}",
                exec(prefix)
            )
        };
        let mine = ours.to_str().unwrap();
        std::fs::write(
            apps.join("wine-extension-mp3.desktop"),
            entry("Winamp", mine),
        )
        .unwrap();
        std::fs::write(
            apps.join("wine-extension-b4s.desktop"),
            entry("Winamp", mine),
        )
        .unwrap();
        std::fs::write(
            apps.join("wine-protocol-ICY.desktop"),
            entry("Winamp", mine),
        )
        .unwrap();
        std::fs::write(
            apps.join("wine-extension-txt.desktop"),
            entry("Outro", "/home/u/.wine"),
        )
        .unwrap();
        std::fs::write(data.join("mime/packages/x-wine-extension-b4s.xml"), "").unwrap();
        std::fs::write(data.join("mime/packages/x-wine-extension-txt.xml"), "").unwrap();
        let program = apps.join("wine/Programs/Winamp/Winamp.desktop");
        let uninstall = apps.join("wine/Programs/Winamp/Desinstalar Winamp.desktop");
        std::fs::write(&program, entry("Winamp", mine)).unwrap();
        std::fs::write(&uninstall, entry("Desinstalar Winamp", mine)).unwrap();

        assert!(tidy_menu(&data, ours));
        assert!(!apps.join("wine-extension-mp3.desktop").exists());
        assert!(!apps.join("wine-protocol-ICY.desktop").exists());
        assert!(
            apps.join("wine-extension-txt.desktop").exists(),
            "outro prefixo fica"
        );
        assert!(!data.join("mime/packages/x-wine-extension-b4s.xml").exists());
        assert!(data.join("mime/packages/x-wine-extension-txt.xml").exists());
        assert!(
            !std::fs::read_to_string(&program)
                .unwrap()
                .contains("NoDisplay")
        );
        assert!(
            std::fs::read_to_string(&uninstall)
                .unwrap()
                .starts_with("[Desktop Entry]\nNoDisplay=true\n")
        );
        assert!(!tidy_menu(&data, ours), "segunda passada não muda nada");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn winetricks_auto_yes_becomes_no() {
        let script = "case x in\n  none)\n    w_info \"Unattended mode, not prompting for confirmation\"\n  ;;\nesac\n";
        let guarded = guarded_winetricks(script).unwrap();
        assert!(
            guarded.contains("w_info \"Unattended mode, not prompting for confirmation\"; false\n")
        );
        assert_eq!(guarded_winetricks("sem a linha"), None);
        let twice = format!("{script}{script}");
        assert_eq!(guarded_winetricks(&twice), None);
    }

    #[test]
    fn installed_winetricks_can_be_guarded() {
        if let Ok(script) = std::fs::read_to_string("/usr/bin/winetricks") {
            assert!(
                guarded_winetricks(&script).is_some(),
                "o winetricks instalado mudou a linha do modo sem perguntas"
            );
        }
    }

    #[test]
    fn every_offered_app_can_be_removed() {
        for id in VERBS
            .iter()
            .copied()
            .chain(RECIPES.iter().map(|recipe| recipe.id))
        {
            assert!(removal(id).is_some(), "{id} sem forma de remover");
        }
    }

    #[test]
    fn uninstall_strings_are_parsed() {
        let text = "HKEY_LOCAL_MACHINE\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Notepad++\n    DisplayName    REG_SZ    Notepad++ (32-bit x86)\n    UninstallString    REG_SZ    C:\\Program Files (x86)\\Notepad++\\uninstall.exe\nHKEY_LOCAL_MACHINE\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Winamp\n    DisplayName    REG_SZ    Winamp\n    UninstallString    REG_SZ    \"C:\\Program Files (x86)\\Winamp\\UninstWA.exe\"\nHKEY_LOCAL_MACHINE\\X\\{0591}\n    DisplayName    REG_SZ    Wine Mono Runtime\n    UninstallString    REG_EXPAND_SZ    MsiExec.exe /I{0591}\n";
        let entries = uninstall_entries(text);
        assert_eq!(entries.len(), 3);
        assert_eq!(
            uninstall_command(&entries[0].1, NSIS),
            (
                "C:\\Program Files (x86)\\Notepad++\\uninstall.exe".to_owned(),
                vec!["/S".to_owned()]
            )
        );
        assert_eq!(
            uninstall_command(&entries[1].1, NSIS),
            (
                "C:\\Program Files (x86)\\Winamp\\UninstWA.exe".to_owned(),
                vec!["/S".to_owned()]
            )
        );
        assert_eq!(
            uninstall_command(&entries[2].1, NSIS),
            (
                "MsiExec.exe".to_owned(),
                vec!["/X{0591}".to_owned(), "/qn".to_owned()]
            )
        );
    }

    #[test]
    fn stale_shortcuts_are_removed() {
        let root = std::env::temp_dir().join(format!("vega-stale-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let prefix = root.join("prefix");
        let data = root.join("share");
        std::fs::create_dir_all(prefix.join("drive_c/windows")).unwrap();
        std::fs::write(prefix.join("drive_c/windows/mspaint.exe"), "").unwrap();
        std::fs::create_dir_all(data.join("applications/wine/Programs/Winamp")).unwrap();
        let live = data.join("applications/lyra-windows-mspaint.desktop");
        let gone = data.join("applications/wine/Programs/Winamp/Winamp.desktop");
        std::fs::write(
            &live,
            windows_desktop_entry(&prefix, &["windows", "mspaint.exe"], "Paint"),
        )
        .unwrap();
        std::fs::write(
            &gone,
            windows_desktop_entry(&prefix, &["ProgramData", "Winamp.lnk"], "Winamp"),
        )
        .unwrap();
        assert_eq!(
            exec_target(
                std::fs::read_to_string(&live)
                    .unwrap()
                    .lines()
                    .find_map(|l| l.strip_prefix("Exec="))
                    .unwrap(),
                &prefix
            ),
            Some(prefix.join("drive_c/windows/mspaint.exe"))
        );
        remove_stale_shortcuts(&data, &prefix);
        assert!(live.exists());
        assert!(!gone.exists());
        assert!(!data.join("applications/wine/Programs/Winamp").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_exe_and_msi_files_run() {
        let dir = std::env::temp_dir().join(format!("vega-run-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, ok) in [
            ("Setup.EXE", true),
            ("pacote.msi", true),
            ("leia.txt", false),
            ("script.bat", false),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, "").unwrap();
            assert_eq!(runnable_file(&path), ok, "{name}");
        }
        assert!(!runnable_file(Path::new("relativo.exe")));
        assert!(!runnable_file(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ahk_verbs_are_offered_verbs() {
        for verb in AHK_VERBS {
            assert!(VERBS.contains(verb), "{verb}");
        }
    }

    #[test]
    fn verbs_are_plain_words() {
        for verb in VERBS {
            assert!(
                verb.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{verb}"
            );
        }
    }
}
