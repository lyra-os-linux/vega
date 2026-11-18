//! Assistente de certificado digital ICP-Brasil, fase 1 (vega#211).
//!
//! Cobre o certificado A1 (arquivo `.pfx`/`.p12`) e o A3 (token ou cartão)
//! que funciona com o OpenSC do Leap (`pcsc-lite`, `pcsc-ccid`, `opensc`). Os
//! middlewares proprietários (SafeNet, Watchdata, GD Burti) ficam na fase 2
//! (vega#263): aqui o token que o OpenSC não reconhece só aparece no
//! diagnóstico.
//!
//! Os pacotes chegam pela transação oficial da loja; o resto roda na conta do
//! usuário, sem root:
//!
//! - lê as leitoras (`opensc-tool -l`), os tokens (`pkcs11-tool -L`) e os
//!   certificados do token, que são públicos e saem sem o PIN;
//! - registra o módulo PKCS#11 do OpenSC (`modutil`) nos bancos NSS dos
//!   navegadores: cada perfil do Firefox e o banco do usuário que o Chromium,
//!   o Edge e os web apps do Vega usam (`~/.pki/nssdb` ou
//!   `~/.local/share/pki/nssdb`; o Chromium 157 usa o que existir);
//! - importa o A1 (`pk12util`) nesses mesmos bancos e avisa da validade.
//!
//! Os navegadores Flatpak não enxergam o módulo do sistema, que fica fora da
//! sandbox: eles só aparecem num aviso.
//!
//! O diagnóstico copiável (para o modelo de lyra-os-linux/.github#5) não leva
//! dados pessoais: sem titular, rótulo do token nem número de série.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use gettextrs::gettext;
use serde::Serialize;

/// Pacotes do `repo-oss` para leitoras e tokens.
pub const PACKAGES: [&str; 3] = ["pcsc-lite", "pcsc-ccid", "opensc"];
/// O `pcscd` sobe pelo socket quando alguém pede uma leitora.
const SOCKET: &str = "pcscd.socket";
/// Módulo PKCS#11 do OpenSC. `VEGA_PKCS11_MODULE` troca o módulo só para
/// testar sem token real (por exemplo, o do SoftHSM).
const OPENSC: &str = "/usr/lib64/opensc-pkcs11.so";
const MODULE_NAME: &str = "OpenSC (Lyra)";
/// Navegadores Flatpak que o Vega oferece e que não enxergam o módulo.
const FLATPAK_BROWSERS: [(&str, &str); 5] = [
    ("com.google.Chrome", "Google Chrome"),
    ("com.brave.Browser", "Brave"),
    ("com.opera.Opera", "Opera"),
    ("com.vivaldi.Vivaldi", "Vivaldi"),
    ("org.mozilla.firefox", "Firefox (Flatpak)"),
];

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Reader {
    pub name: String,
    /// Há cartão ou token presente na leitora.
    pub card: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Token {
    /// Nome da leitora (o `pkcs11-tool` mostra junto do slot).
    pub reader: String,
    pub manufacturer: String,
    pub model: String,
    /// Rótulo do token; costuma ter o nome do titular, então não vai para o
    /// diagnóstico.
    pub label: String,
    /// `false` quando a leitora tem um cartão que o OpenSC não reconhece.
    pub recognized: bool,
    #[serde(skip)]
    pub slot: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub struct Certificate {
    /// Nome do titular, sem o CPF/CNPJ que a ICP-Brasil põe depois de ":".
    pub holder: String,
    pub issuer: String,
    /// Validade, `AAAA-MM-DD`.
    pub not_after: String,
    pub days_left: i64,
    /// `token` (A3) ou `a1`.
    pub kind: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Database {
    /// "Firefox (perfil)" ou "Chromium, Edge e web apps".
    pub browser: String,
    #[serde(skip)]
    pub path: PathBuf,
    /// O módulo do OpenSC já está no banco.
    pub registered: bool,
}

#[derive(Debug, Serialize)]
pub struct CertificateStatus {
    /// Pacotes de `PACKAGES` que faltam.
    pub missing: Vec<&'static str>,
    /// `systemctl is-enabled pcscd.socket`.
    pub pcscd: String,
    pub readers: Vec<Reader>,
    pub tokens: Vec<Token>,
    pub certificates: Vec<Certificate>,
    pub databases: Vec<Database>,
    /// Navegadores Flatpak instalados, que não usam o token.
    pub flatpak: Vec<&'static str>,
}

fn module() -> String {
    std::env::var("VEGA_PKCS11_MODULE").unwrap_or_else(|_| OPENSC.to_owned())
}

/// Roda com saída em inglês, que é o que os analisadores esperam.
fn run(program: &str, args: &[&str]) -> Option<(bool, Vec<u8>)> {
    let output = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    Some((output.status.success(), output.stdout))
}

fn run_text(program: &str, args: &[&str]) -> Option<(bool, String)> {
    run(program, args).map(|(ok, out)| (ok, String::from_utf8_lossy(&out).into_owned()))
}

/// Roda com `input` na entrada padrão (senha ou certificado), sem expor nada
/// na linha de comando.
fn run_with_input(program: &str, args: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{program}: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Erro de escrita aparece no código de saída.
        let _ = stdin.write_all(input);
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("{program}: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

// ---------------------------------------------------------------------------
// Analisadores (puros, testados com saídas reais)

/// Leitoras do `opensc-tool -l`. As colunas seguem o cabeçalho
/// `Nr.  Card  Features  Name`.
pub fn parse_readers(output: &str) -> Vec<Reader> {
    let mut lines = output.lines();
    let Some(header) = lines.by_ref().find(|line| line.starts_with("Nr.")) else {
        return Vec::new();
    };
    let (Some(card), Some(name)) = (header.find("Card"), header.find("Name")) else {
        return Vec::new();
    };
    lines
        .filter(|line| line.len() > name)
        .map(|line| Reader {
            name: line[name..].trim().to_owned(),
            card: line.get(card..card + 3) == Some("Yes"),
        })
        .collect()
}

/// Tokens do `pkcs11-tool -L`. Slots vazios ficam de fora; cartão que o
/// módulo não reconhece entra com `recognized: false`.
pub fn parse_tokens(output: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current: Option<Token> = None;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Slot ") {
            tokens.extend(current.take());
            // "0 (0x0): Leitora" ou "0 (0x0): Leitora\n  (empty)"
            let (slot, reader) = rest.split_once(": ").unwrap_or((rest, ""));
            let slot = slot
                .split_once('(')
                .map(|(_, id)| id.trim_end_matches(')'))
                .unwrap_or(slot)
                .trim()
                .to_owned();
            current = Some(Token {
                reader: reader.trim().to_owned(),
                manufacturer: String::new(),
                model: String::new(),
                label: String::new(),
                recognized: false,
                slot,
            });
            continue;
        }
        let Some(token) = current.as_mut() else {
            continue;
        };
        let line = line.trim();
        // Slot sem cartão ou com cartão em branco: nada a usar nem relatar.
        if line == "(empty)" || line.starts_with("token state:") && line.ends_with("uninitialized")
        {
            current = None;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_owned();
        match key.trim() {
            "token label" => {
                token.label = value;
                token.recognized = true;
            }
            "token manufacturer" => token.manufacturer = value,
            "token model" => token.model = value,
            _ => {}
        }
    }
    tokens.extend(current);
    tokens
}

/// IDs dos certificados do `pkcs11-tool -O --type cert`.
pub fn parse_certificate_ids(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("ID:"))
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .collect()
}

/// Primeiro `CN=` de um nome no formato RFC 2253 (`CN=...,OU=...`), sem o
/// escape de vírgula.
fn common_name(dn: &str) -> String {
    let rest = match dn.strip_prefix("CN=") {
        Some(rest) => rest,
        None => match dn.find(",CN=") {
            Some(at) => &dn[at + 4..],
            None => return dn.to_owned(),
        },
    };
    let mut value = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => value.extend(chars.next()),
            ',' => break,
            _ => value.push(c),
        }
    }
    value
}

/// Titular sem o documento: a ICP-Brasil usa `NOME:CPF` ou `EMPRESA:CNPJ`.
pub fn holder_name(cn: &str) -> String {
    match cn.rsplit_once(':') {
        Some((name, doc)) if !doc.is_empty() && doc.chars().all(|c| c.is_ascii_digit()) => {
            name.trim().to_owned()
        }
        _ => cn.trim().to_owned(),
    }
}

/// Dias do calendário civil desde 1970-01-01 (algoritmo de Howard Hinnant).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn parse_date(date: &str) -> Option<i64> {
    let mut parts = date.get(..10)?.split('-').map(str::parse::<i64>);
    let (year, month, day) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    Some(days_from_civil(year, month, day))
}

fn today() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    secs.div_euclid(86_400)
}

/// Saída de `openssl x509 -noout -subject -issuer -enddate -nameopt RFC2253
/// -dateopt iso_8601`.
pub fn parse_x509(output: &str, kind: &'static str, today: i64) -> Option<Certificate> {
    let field = |name: &str| {
        output
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .map(str::trim)
    };
    let not_after = field("notAfter=")?.get(..10)?.to_owned();
    let days_left = parse_date(&not_after)? - today;
    Some(Certificate {
        holder: holder_name(&common_name(field("subject=")?)),
        issuer: holder_name(&common_name(field("issuer=").unwrap_or_default())),
        not_after,
        days_left,
        kind,
    })
}

/// Bancos NSS com o módulo: o `pkcs11.txt` de cada banco lista as
/// bibliotecas carregadas.
pub fn has_module(pkcs11_txt: &str, library: &str) -> bool {
    pkcs11_txt
        .lines()
        .any(|line| line.trim().strip_prefix("library=") == Some(library))
}

/// Pastas dos perfis no `profiles.ini` do Firefox.
pub fn parse_profiles(ini: &str, base: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    let mut path: Option<String> = None;
    let mut relative = true;
    let mut flush = |path: &mut Option<String>, relative: &mut bool| {
        if let Some(p) = path.take() {
            profiles.push(if *relative {
                base.join(p)
            } else {
                PathBuf::from(p)
            });
        }
        *relative = true;
    };
    for line in ini.lines().map(str::trim) {
        if line.starts_with('[') {
            flush(&mut path, &mut relative);
        } else if let Some(value) = line.strip_prefix("Path=") {
            path = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("IsRelative=") {
            relative = value != "0";
        }
    }
    flush(&mut path, &mut relative);
    profiles
}

/// Apelidos dos certificados com chave privada (`u,u,u`) no `certutil -L`.
pub fn parse_user_certs(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            let (name, flags) = line.rsplit_once(char::is_whitespace)?;
            flags.starts_with('u').then(|| name.trim().to_owned())
        })
        .filter(|name| !name.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Leitura do sistema

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Banco do Chromium: o do XDG se existir, senão o `~/.pki/nssdb`. Sem
/// nenhum dos dois, o do XDG, que é o que o Chromium cria.
fn chromium_dbs(home: &Path) -> Vec<PathBuf> {
    let xdg = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("pki/nssdb");
    let legacy = home.join(".pki/nssdb");
    let mut dbs: Vec<PathBuf> = Vec::new();
    let mut seen = HashSet::new();
    for db in [&xdg, &legacy] {
        if db.join("cert9.db").exists() {
            let real = std::fs::canonicalize(db).unwrap_or_else(|_| db.clone());
            if seen.insert(real) {
                dbs.push(db.clone());
            }
        }
    }
    if dbs.is_empty() {
        dbs.push(xdg);
    }
    dbs
}

fn firefox_profiles(home: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    for base in [
        home.join(".config/mozilla/firefox"),
        home.join(".mozilla/firefox"),
    ] {
        if let Ok(ini) = std::fs::read_to_string(base.join("profiles.ini")) {
            profiles.extend(
                parse_profiles(&ini, &base)
                    .into_iter()
                    .filter(|profile| profile.join("cert9.db").exists()),
            );
        }
    }
    profiles
}

fn databases() -> Vec<Database> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let library = module();
    let registered = |path: &Path| {
        std::fs::read_to_string(path.join("pkcs11.txt"))
            .map(|txt| has_module(&txt, &library))
            .unwrap_or(false)
    };
    let mut dbs: Vec<Database> = firefox_profiles(&home)
        .into_iter()
        .map(|path| {
            let profile = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            Database {
                browser: format!("Firefox ({profile})"),
                registered: registered(&path),
                path,
            }
        })
        .collect();
    dbs.extend(chromium_dbs(&home).into_iter().map(|path| Database {
        browser: gettext("Chromium, Edge e web apps do Vega"),
        registered: registered(&path),
        path,
    }));
    dbs
}

fn missing_packages() -> Vec<&'static str> {
    PACKAGES
        .into_iter()
        .filter(|name| !matches!(run("rpm", &["-q", name]), Some((true, _))))
        .collect()
}

fn installed_flatpak_browsers() -> Vec<&'static str> {
    let Some((true, list)) = run_text("flatpak", &["list", "--app", "--columns=application"])
    else {
        return Vec::new();
    };
    let ids: HashSet<&str> = list.lines().map(str::trim).collect();
    FLATPAK_BROWSERS
        .into_iter()
        .filter(|(id, _)| ids.contains(id))
        .map(|(_, name)| name)
        .collect()
}

fn x509_info(pem_or_der: &[u8], der: bool, kind: &'static str) -> Option<Certificate> {
    let mut args = vec![
        "x509", "-noout", "-subject", "-issuer", "-enddate", "-nameopt", "RFC2253", "-dateopt",
        "iso_8601",
    ];
    if der {
        args.extend(["-inform", "DER"]);
    }
    let out = run_with_input("openssl", &args, pem_or_der).ok()?;
    parse_x509(&String::from_utf8_lossy(&out), kind, today())
}

fn token_certificates(library: &str, slot: &str) -> Vec<Certificate> {
    let Some((true, list)) = run_text(
        "pkcs11-tool",
        &["--module", library, "--slot", slot, "-O", "--type", "cert"],
    ) else {
        return Vec::new();
    };
    parse_certificate_ids(&list)
        .iter()
        .filter_map(|id| {
            let (ok, der) = run(
                "pkcs11-tool",
                &[
                    "--module",
                    library,
                    "--slot",
                    slot,
                    "--read-object",
                    "--type",
                    "cert",
                    "--id",
                    id,
                    "-o",
                    "/dev/stdout",
                ],
            )?;
            if !ok {
                return None;
            }
            x509_info(&der, true, "token")
        })
        .collect()
}

/// Certificados A1 já importados no banco do Chromium.
fn a1_certificates(db: &Path) -> Vec<Certificate> {
    let dir = format!("sql:{}", db.display());
    let Some((true, list)) = run_text("certutil", &["-L", "-d", &dir]) else {
        return Vec::new();
    };
    parse_user_certs(&list)
        .iter()
        .filter_map(|nick| {
            let (ok, pem) = run("certutil", &["-L", "-d", &dir, "-n", nick, "-a"])?;
            if !ok {
                return None;
            }
            x509_info(&pem, false, "a1")
        })
        .collect()
}

/// Estado do assistente. Só lê; roda sem root.
#[tauri::command]
pub async fn certificate_status() -> CertificateStatus {
    tauri::async_runtime::spawn_blocking(status)
        .await
        .unwrap_or_else(|_| status())
}

fn status() -> CertificateStatus {
    let missing = missing_packages();
    let pcscd = run_text("systemctl", &["is-enabled", SOCKET])
        .map(|(_, state)| state.trim().to_owned())
        .unwrap_or_default();
    let library = module();
    let have_tools = Path::new(&library).exists();
    let readers = run_text("opensc-tool", &["-l"])
        .map(|(_, out)| parse_readers(&out))
        .unwrap_or_default();
    let tokens = if have_tools {
        run_text("pkcs11-tool", &["--module", &library, "-L"])
            .map(|(_, out)| parse_tokens(&out))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut certificates: Vec<Certificate> = tokens
        .iter()
        .filter(|token| token.recognized)
        .flat_map(|token| token_certificates(&library, &token.slot))
        .collect();
    let databases = databases();
    if let Some(db) = databases
        .iter()
        .find(|db| !db.browser.starts_with("Firefox"))
    {
        certificates.extend(a1_certificates(&db.path));
    }
    CertificateStatus {
        missing,
        pcscd,
        readers,
        tokens,
        certificates,
        databases,
        flatpak: installed_flatpak_browsers(),
    }
}

// ---------------------------------------------------------------------------
// Ações na conta do usuário

fn ensure_db(path: &Path) -> Result<(), String> {
    if path.join("cert9.db").exists() {
        return Ok(());
    }
    std::fs::create_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let dir = format!("sql:{}", path.display());
    match run_text("certutil", &["-N", "-d", &dir, "--empty-password"]) {
        Some((true, _)) => Ok(()),
        _ => Err(
            gettext("Não foi possível criar o banco de certificados em {path}.")
                .replace("{path}", &path.display().to_string()),
        ),
    }
}

/// Registra o módulo do OpenSC nos bancos que ainda não o têm. Devolve os
/// navegadores em que registrou. O Firefox só vê o módulo depois de reabrir.
#[tauri::command]
pub async fn certificate_register_browsers() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let library = module();
        if !Path::new(&library).exists() {
            return Err(gettext("O OpenSC não está instalado."));
        }
        let mut done = Vec::new();
        for db in databases().into_iter().filter(|db| !db.registered) {
            ensure_db(&db.path)?;
            let dir = format!("sql:{}", db.path.display());
            // `-force` dispensa a pergunta interativa sobre o navegador aberto.
            match run_text(
                "modutil",
                &[
                    "-dbdir",
                    &dir,
                    "-add",
                    MODULE_NAME,
                    "-libfile",
                    &library,
                    "-force",
                ],
            ) {
                Some((true, _)) => done.push(db.browser),
                _ => {
                    return Err(gettext("Não foi possível registrar o token em {browser}.")
                        .replace("{browser}", &db.browser));
                }
            }
        }
        Ok(done)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Lê o A1 com a senha. `-legacy` aceita também os `.pfx` antigos (RC2/3DES),
/// comuns entre os emitidos pelas ACs.
fn read_a1(path: &Path, password: &str) -> Result<Certificate, String> {
    let file = path.to_string_lossy();
    let pem = run_with_input(
        "openssl",
        &[
            "pkcs12", "-legacy", "-in", &file, "-nokeys", "-clcerts", "-passin", "stdin",
        ],
        format!("{password}\n").as_bytes(),
    )
    .map_err(|error| {
        if error.contains("invalid password")
            || error.contains("mac verify")
            || error.contains("Mac verify")
        {
            gettext("Senha incorreta.")
        } else {
            gettext("O arquivo não é um certificado A1 válido.")
        }
    })?;
    x509_info(&pem, false, "a1").ok_or_else(|| gettext("O arquivo não é um certificado A1 válido."))
}

/// Importa o A1 no banco do Chromium e em cada perfil do Firefox. A senha
/// vai para o `pk12util` por um arquivo temporário só do usuário, apagado em
/// seguida, e nunca pela linha de comando.
#[tauri::command]
pub async fn certificate_import_a1(path: String, password: String) -> Result<Certificate, String> {
    tauri::async_runtime::spawn_blocking(move || import_a1(Path::new(&path), &password))
        .await
        .map_err(|error| error.to_string())?
}

fn import_a1(path: &Path, password: &str) -> Result<Certificate, String> {
    let certificate = read_a1(path, password)?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let secret = runtime.join(format!("vega-a1-{}", std::process::id()));
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&secret)
            .map_err(|error| error.to_string())?;
        file.write_all(password.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let result = (|| {
        for db in databases() {
            ensure_db(&db.path)?;
            let dir = format!("sql:{}", db.path.display());
            let file = path.to_string_lossy();
            let secret = secret.to_string_lossy();
            match run_text(
                "pk12util",
                &["-i", &file, "-d", &dir, "-w", &secret, "-K", ""],
            ) {
                Some((true, _)) => {}
                _ => {
                    return Err(
                        gettext("Não foi possível importar o certificado em {browser}.")
                            .replace("{browser}", &db.browser),
                    );
                }
            }
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&secret);
    result.map(|()| certificate)
}

/// Texto para relatar problema, sem dados pessoais.
pub fn diagnostic_text(status: &CertificateStatus) -> String {
    let mut lines = vec!["Lyra OS — diagnóstico do certificado digital (Vega)".to_owned()];
    lines.push(format!(
        "Pacotes: {}",
        if status.missing.is_empty() {
            "pcsc-lite, pcsc-ccid e opensc instalados".to_owned()
        } else {
            format!("faltam {}", status.missing.join(", "))
        }
    ));
    lines.push(format!(
        "pcscd.socket: {}",
        if status.pcscd.is_empty() {
            "?"
        } else {
            &status.pcscd
        }
    ));
    if status.readers.is_empty() {
        lines.push("Leitoras: nenhuma".to_owned());
    }
    for reader in &status.readers {
        lines.push(format!(
            "Leitora: {} ({})",
            reader.name,
            if reader.card {
                "com cartão"
            } else {
                "sem cartão"
            }
        ));
    }
    for token in &status.tokens {
        if token.recognized {
            lines.push(format!(
                "Token: {} {} — reconhecido pelo OpenSC",
                token.manufacturer, token.model
            ));
        } else {
            lines.push(format!(
                "Token em {}: não reconhecido pelo OpenSC",
                token.reader
            ));
        }
    }
    let count = |kind: &str| {
        status
            .certificates
            .iter()
            .filter(|c| c.kind == kind)
            .count()
    };
    lines.push(format!(
        "Certificados: {} no token, {} A1",
        count("token"),
        count("a1")
    ));
    for db in &status.databases {
        lines.push(format!(
            "{}: {}",
            db.browser,
            if db.registered {
                "token registrado"
            } else {
                "token não registrado"
            }
        ));
    }
    if !status.flatpak.is_empty() {
        lines.push(format!(
            "Navegadores Flatpak (sem acesso ao token): {}",
            status.flatpak.join(", ")
        ));
    }
    lines.join("\n")
}

#[tauri::command]
pub async fn certificate_diagnostics() -> String {
    tauri::async_runtime::spawn_blocking(|| diagnostic_text(&status()))
        .await
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_follow_the_header_columns() {
        let out = "# Detected readers (pcsc)\n\
                   Nr.  Card  Features  Name\n\
                   0    Yes             Alcor Micro AU9540 00 00\n\
                   1    No    PIN pad   Gemalto PC Twin Reader 01 00\n";
        assert_eq!(
            parse_readers(out),
            vec![
                Reader {
                    name: "Alcor Micro AU9540 00 00".into(),
                    card: true
                },
                Reader {
                    name: "Gemalto PC Twin Reader 01 00".into(),
                    card: false
                },
            ]
        );
        assert!(parse_readers("No smart card readers found.\n").is_empty());
    }

    #[test]
    fn tokens_skip_empty_slots_and_flag_unknown_cards() {
        let out = "Available slots:\n\
                   Slot 0 (0x0): Alcor Micro AU9540 00 00\n\
                   \x20 token label        : FULANO DE TAL\n\
                   \x20 token manufacturer : Certisign\n\
                   \x20 token model        : PKCS#15 emulated\n\
                   \x20 serial num         : 1234\n\
                   Slot 1 (0x4): Gemalto PC Twin Reader 01 00\n\
                   \x20 (empty)\n\
                   Slot 2 (0x8): SafeNet eToken 5100 [eToken 5110 SC] 02 00\n\
                   \x20 (token not recognized)\n\
                   Slot 3 (0x1): SoftHSM slot ID 0x1\n\
                   \x20 token state:   uninitialized\n";
        let tokens = parse_tokens(out);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].slot, "0x0");
        assert_eq!(tokens[0].manufacturer, "Certisign");
        assert!(tokens[0].recognized);
        assert_eq!(
            tokens[1].reader,
            "SafeNet eToken 5100 [eToken 5110 SC] 02 00"
        );
        assert!(!tokens[1].recognized);
    }

    #[test]
    fn certificate_ids_come_from_the_object_list() {
        let out = "Certificate Object; type = X.509 cert\n  label:      Cert\n  subject:    DN: CN=X\n  ID:         01\n\
                   Certificate Object; type = X.509 cert\n  label:      CA\n  ID:         a1b2\n";
        assert_eq!(parse_certificate_ids(out), vec!["01", "a1b2"]);
    }

    #[test]
    fn holder_drops_the_cpf_and_keeps_names_with_colons() {
        assert_eq!(holder_name("FULANO DE TAL:12345678901"), "FULANO DE TAL");
        assert_eq!(holder_name("EMPRESA LTDA:12345678000199"), "EMPRESA LTDA");
        assert_eq!(holder_name("Teste: sem documento"), "Teste: sem documento");
    }

    #[test]
    fn x509_output_gives_holder_issuer_and_days_left() {
        let out = "subject=CN=FULANO DE TAL:12345678901,OU=Teste Lyra,O=ICP-Brasil,C=BR\n\
                   issuer=CN=AC Teste v5,O=ICP-Brasil,C=BR\n\
                   notAfter=2027-12-23 12:20:45Z\n";
        let today = days_from_civil(2026, 11, 18);
        let cert = parse_x509(out, "a1", today).unwrap();
        assert_eq!(cert.holder, "FULANO DE TAL");
        assert_eq!(cert.issuer, "AC Teste v5");
        assert_eq!(cert.not_after, "2027-12-23");
        assert_eq!(cert.days_left, 400);
        let escaped = "subject=CN=Empresa\\, Filial:123,O=X\nnotAfter=2026-11-17 00:00:00Z\n";
        let cert = parse_x509(escaped, "token", today).unwrap();
        assert_eq!(cert.holder, "Empresa, Filial");
        assert_eq!(cert.days_left, -1);
    }

    #[test]
    fn calendar_days_match_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(
            days_from_civil(2028, 2, 29) - days_from_civil(2028, 2, 28),
            1
        );
    }

    #[test]
    fn module_is_found_in_pkcs11_txt() {
        let txt = "library=\nname=NSS Internal PKCS #11 Module\n\nlibrary=/usr/lib64/opensc-pkcs11.so\nname=OpenSC (Lyra)\n";
        assert!(has_module(txt, OPENSC));
        assert!(!has_module("library=\nname=NSS\n", OPENSC));
    }

    #[test]
    fn firefox_profiles_resolve_relative_and_absolute_paths() {
        let ini = "[Profile1]\nName=default\nIsRelative=1\nPath=2wx451if.default\n\n\
                   [Profile0]\nName=outro\nIsRelative=0\nPath=/srv/perfil\n\n[General]\nVersion=2\n";
        let base = Path::new("/home/u/.config/mozilla/firefox");
        assert_eq!(
            parse_profiles(ini, base),
            vec![base.join("2wx451if.default"), PathBuf::from("/srv/perfil")]
        );
    }

    #[test]
    fn only_certificates_with_private_key_count_as_a1() {
        let out = "\nCertificate Nickname                                         Trust Attributes\n\
                   \x20                                                            SSL,S/MIME,JAR/XPI\n\n\
                   FULANO DE TAL:12345678901                                    u,u,u\n\
                   AC Raiz                                                      CT,C,C\n";
        assert_eq!(parse_user_certs(out), vec!["FULANO DE TAL:12345678901"]);
    }

    /// Medição na máquina, com token simulado:
    /// `VEGA_PKCS11_MODULE=/usr/lib64/pkcs11/libsofthsm2.so cargo test live_status -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_status() {
        let status = status();
        println!("{}", serde_json::to_string_pretty(&status).unwrap());
        println!("{}", diagnostic_text(&status));
    }

    #[test]
    fn diagnostic_has_no_personal_data() {
        let status = CertificateStatus {
            missing: vec![],
            pcscd: "enabled".into(),
            readers: vec![Reader {
                name: "Leitora X".into(),
                card: true,
            }],
            tokens: vec![Token {
                reader: "Leitora X".into(),
                manufacturer: "Certisign".into(),
                model: "PKCS#15".into(),
                label: "FULANO DE TAL".into(),
                recognized: true,
                slot: "0x0".into(),
            }],
            certificates: vec![Certificate {
                holder: "FULANO DE TAL".into(),
                issuer: "AC".into(),
                not_after: "2027-01-01".into(),
                days_left: 10,
                kind: "token",
            }],
            databases: vec![],
            flatpak: vec!["Google Chrome"],
        };
        let text = diagnostic_text(&status);
        assert!(!text.contains("FULANO"));
        assert!(text.contains("Certisign"));
        assert!(text.contains("Google Chrome"));
    }
}
