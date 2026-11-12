//! Download guiado pelo navegador (vega#208).
//!
//! Alguns fabricantes só entregam o instalador depois do login no site deles
//! (Oracle, SAP, DaVinci Resolve). O Vega não vê credenciais: abre a página
//! no navegador, espera o arquivo chegar na pasta de downloads e só aceita o
//! que bate com o SHA-256 conferido pelo Lyra. Um arquivo diferente é
//! recusado com explicação; uma versão que o Lyra ainda não conferiu também.
//!
//! O componente é único para que todos os cartões tenham o mesmo
//! comportamento e a mesma conferência. Quem baixa direto (o IRPF, que a
//! Receita publica sem login) usa só a conferência ([`verify`]). O diálogo
//! na interface chega com o primeiro cartão que exige login (Oracle, #189).
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gettextrs::gettext;
use sha2::{Digest, Sha256};

/// Arquivo esperado, fixado no catálogo do Vega.
#[derive(Debug, Clone, Copy)]
pub struct Expected {
    /// Página do fabricante aberta no navegador.
    pub page: &'static str,
    /// Nome do arquivo; `*` vale qualquer trecho (`jdk-*_linux-x64_bin.rpm`).
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    /// Versão conferida pelo Lyra, mostrada na interface.
    pub version: &'static str,
}

/// Por que um arquivo foi recusado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Tamanho diferente: em geral, outra versão.
    UnknownVersion,
    /// Mesmo tamanho, conteúdo diferente.
    WrongChecksum,
    Unreadable(String),
}

impl Rejection {
    pub fn message(&self, expected: &Expected) -> String {
        match self {
            Self::UnknownVersion => gettext(
                "Esta versão ainda não foi conferida pelo Lyra. O Vega aceita só a versão {version}.",
            )
            .replace("{version}", expected.version),
            Self::WrongChecksum => gettext(
                "O arquivo não confere com o checksum conferido pelo Lyra; ele pode estar corrompido ou ter sido alterado. Nada foi instalado.",
            ),
            Self::Unreadable(error) => error.clone(),
        }
    }
}

/// Nome com `*` como curinga.
pub fn name_matches(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if name.len() < first.len() + last.len() || !name.starts_with(first) || !name.ends_with(last) {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

/// Tira o ` (1)` que o navegador acrescenta quando o arquivo já existe
/// (`nome (1).ext`, ou `nome.sh (1).bin` no Chrome).
fn without_copy_suffix(name: &str) -> String {
    if let Some(open) = name.rfind(" (")
        && let Some(close) = name[open..].find(')')
    {
        let number = &name[open + 2..open + close];
        if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
            return format!("{}{}", &name[..open], &name[open + close + 1..]);
        }
    }
    name.to_owned()
}

/// O nome é do arquivo esperado, inclusive numa cópia `nome (1).ext`.
pub fn is_candidate(expected: &Expected, name: &str) -> bool {
    !is_partial(name)
        && (name_matches(expected.file, name)
            || name_matches(expected.file, &without_copy_suffix(name)))
}

/// Download ainda em andamento no Firefox, no Chrome ou no Epiphany.
pub fn is_partial(name: &str) -> bool {
    [".part", ".crdownload", ".download", ".tmp"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 20];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Confere tamanho e SHA-256. Não olha o nome: o arquivo escolhido à mão
/// pode ter sido renomeado.
pub fn verify(expected: &Expected, path: &Path) -> Result<(), Rejection> {
    let size = std::fs::metadata(path)
        .map_err(|error| Rejection::Unreadable(error.to_string()))?
        .len();
    if size != expected.size {
        return Err(Rejection::UnknownVersion);
    }
    let hash = sha256_file(path).map_err(Rejection::Unreadable)?;
    if !hash.eq_ignore_ascii_case(expected.sha256) {
        return Err(Rejection::WrongChecksum);
    }
    Ok(())
}

/// Resultado de uma rodada do observador.
#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    Waiting,
    Accepted(PathBuf),
    /// Arquivo com o nome certo que não passou na conferência. É avisado uma
    /// vez só; o observador continua esperando o arquivo certo.
    Rejected(PathBuf, Rejection),
}

/// Observa a pasta de downloads. Um arquivo só é conferido quando o tamanho
/// fica igual em duas rodadas seguidas, sem o `.part` do navegador ao lado.
pub struct Watcher {
    dir: PathBuf,
    expected: Expected,
    sizes: HashMap<PathBuf, u64>,
    rejected: HashSet<PathBuf>,
}

impl Watcher {
    pub fn new(dir: PathBuf, expected: Expected) -> Self {
        Self {
            dir,
            expected,
            sizes: HashMap::new(),
            rejected: HashSet::new(),
        }
    }

    pub fn poll(&mut self) -> Poll {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Poll::Waiting;
        };
        let names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        let mut candidates: Vec<&String> = names
            .iter()
            .filter(|name| is_candidate(&self.expected, name))
            .collect();
        candidates.sort();
        let mut sizes = HashMap::new();
        let mut outcome = Poll::Waiting;
        for name in candidates {
            let path = self.dir.join(name);
            if self.rejected.contains(&path) {
                continue;
            }
            let in_progress = names.iter().any(|other| {
                other != name && other.starts_with(name.as_str()) && is_partial(other)
            });
            let Ok(size) = std::fs::metadata(&path).map(|meta| meta.len()) else {
                continue;
            };
            sizes.insert(path.clone(), size);
            let stable = size > 0 && self.sizes.get(&path) == Some(&size);
            if in_progress || !stable || outcome != Poll::Waiting {
                continue;
            }
            outcome = match verify(&self.expected, &path) {
                Ok(()) => Poll::Accepted(path),
                Err(rejection) => {
                    self.rejected.insert(path.clone());
                    Poll::Rejected(path, rejection)
                }
            };
        }
        self.sizes = sizes;
        outcome
    }
}

/// Pasta de downloads do usuário (`xdg-user-dir DOWNLOAD`).
pub fn downloads_dir() -> Option<PathBuf> {
    let mut command = std::process::Command::new("xdg-user-dir");
    command.arg("DOWNLOAD");
    crate::webkit_env::clean_command(&mut command);
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads")))
}

/// Como o fluxo terminou.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Accepted(PathBuf),
    Cancelled,
    TimedOut,
}

/// Espera o arquivo conferido. `on_rejected` avisa a interface de cada
/// arquivo recusado; `cancel` encerra a espera (botão Cancelar, ou o arquivo
/// escolhido à mão já aceito).
pub fn wait(
    watcher: &mut Watcher,
    cancel: &Arc<AtomicBool>,
    interval: Duration,
    timeout: Duration,
    on_rejected: &dyn Fn(&Path, &Rejection),
) -> Outcome {
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Outcome::Cancelled;
        }
        match watcher.poll() {
            Poll::Accepted(path) => return Outcome::Accepted(path),
            Poll::Rejected(path, rejection) => on_rejected(&path, &rejection),
            Poll::Waiting => {}
        }
        if started.elapsed() >= timeout {
            return Outcome::TimedOut;
        }
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &[u8] = b"instalador conferido pelo Lyra";

    fn expected() -> Expected {
        Expected {
            page: "https://example.com/download",
            file: "tool-*-linux-x64.sh.bin",
            size: CONTENT.len() as u64,
            sha256: "",
            version: "1.5",
        }
    }

    fn expected_with_hash() -> Expected {
        let hash: String = Sha256::digest(CONTENT)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Expected {
            sha256: Box::leak(hash.into_boxed_str()),
            ..expected()
        }
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vega-guided-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn wildcard_and_browser_copies_match() {
        let expected = expected();
        assert!(is_candidate(&expected, "tool-1.5-linux-x64.sh.bin"));
        assert!(is_candidate(&expected, "tool-1.5-linux-x64 (1).sh.bin"));
        assert!(is_candidate(&expected, "tool-1.5-linux-x64.sh (2).bin"));
        assert!(!is_candidate(&expected, "tool-1.5-linux-x64.sh.bin.part"));
        assert!(!is_candidate(
            &expected,
            "tool-1.5-linux-x64.sh.bin.crdownload"
        ));
        assert!(!is_candidate(&expected, "other-1.5-linux-x64.sh.bin"));
        assert!(name_matches("exato.zip", "exato.zip"));
        assert!(!name_matches("exato.zip", "exato.zip.zip"));
    }

    #[test]
    fn partial_file_waits_until_the_browser_finishes() {
        let dir = dir("partial");
        let file = dir.join("tool-1.5-linux-x64.sh.bin");
        let mut watcher = Watcher::new(dir.clone(), expected_with_hash());
        // Firefox: o arquivo final existe vazio enquanto o .part cresce.
        std::fs::write(&file, b"").unwrap();
        std::fs::write(dir.join("tool-1.5-linux-x64.sh.bin.part"), &CONTENT[..5]).unwrap();
        assert_eq!(watcher.poll(), Poll::Waiting);
        assert_eq!(watcher.poll(), Poll::Waiting);
        std::fs::remove_file(dir.join("tool-1.5-linux-x64.sh.bin.part")).unwrap();
        std::fs::write(&file, CONTENT).unwrap();
        // Tamanho ainda não estável: a primeira rodada só anota.
        assert_eq!(watcher.poll(), Poll::Waiting);
        assert_eq!(watcher.poll(), Poll::Accepted(file));
    }

    #[test]
    fn right_name_with_wrong_checksum_is_refused_once() {
        let dir = dir("checksum");
        let file = dir.join("tool-1.5-linux-x64.sh.bin");
        let mut forged = CONTENT.to_vec();
        forged[0] ^= 1;
        std::fs::write(&file, &forged).unwrap();
        let mut watcher = Watcher::new(dir.clone(), expected_with_hash());
        assert_eq!(watcher.poll(), Poll::Waiting);
        assert_eq!(
            watcher.poll(),
            Poll::Rejected(file.clone(), Rejection::WrongChecksum)
        );
        assert_eq!(watcher.poll(), Poll::Waiting);
        std::fs::write(
            dir.join("tool-1.6-linux-x64.sh.bin"),
            b"outra versao, maior",
        )
        .unwrap();
        watcher.poll();
        assert!(matches!(
            watcher.poll(),
            Poll::Rejected(_, Rejection::UnknownVersion)
        ));
    }

    #[test]
    fn two_downloads_at_once_accept_the_one_that_matches() {
        let dir = dir("two");
        let good = dir.join("tool-1.5-linux-x64 (1).sh.bin");
        let bad = dir.join("tool-1.5-linux-x64.sh.bin");
        let mut forged = CONTENT.to_vec();
        forged[1] ^= 1;
        std::fs::write(&bad, &forged).unwrap();
        std::fs::write(&good, CONTENT).unwrap();
        let mut watcher = Watcher::new(dir.clone(), expected_with_hash());
        assert_eq!(watcher.poll(), Poll::Waiting);
        // Ordem alfabética: "… (1).sh.bin" vem antes e é o certo.
        assert_eq!(watcher.poll(), Poll::Accepted(good));
        // O errado não passa numa rodada seguinte.
        std::fs::remove_file(dir.join("tool-1.5-linux-x64 (1).sh.bin")).unwrap();
        assert_eq!(
            watcher.poll(),
            Poll::Rejected(bad, Rejection::WrongChecksum)
        );
    }

    #[test]
    fn cancel_and_timeout_end_the_wait() {
        let dir = dir("cancel");
        let mut watcher = Watcher::new(dir.clone(), expected_with_hash());
        let cancel = Arc::new(AtomicBool::new(true));
        assert_eq!(
            wait(
                &mut watcher,
                &cancel,
                Duration::ZERO,
                Duration::from_secs(60),
                &|_, _| {}
            ),
            Outcome::Cancelled
        );
        cancel.store(false, Ordering::SeqCst);
        assert_eq!(
            wait(
                &mut watcher,
                &cancel,
                Duration::from_millis(1),
                Duration::from_millis(5),
                &|_, _| {}
            ),
            Outcome::TimedOut
        );
    }

    #[test]
    fn chosen_file_is_checked_by_content_not_name() {
        let dir = dir("chosen");
        let file = dir.join("renomeado.bin");
        std::fs::write(&file, CONTENT).unwrap();
        assert_eq!(verify(&expected_with_hash(), &file), Ok(()));
        assert_eq!(
            verify(&expected_with_hash(), &dir.join("nao-existe")),
            Err(Rejection::Unreadable(
                std::fs::metadata(dir.join("nao-existe"))
                    .unwrap_err()
                    .to_string()
            ))
        );
    }
}
