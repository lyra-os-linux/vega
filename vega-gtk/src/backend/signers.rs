//! Assinadores digitais ICP-Brasil na conta do usuário (vega#214).
//!
//! O Assinador Serpro é publicado pelo Serpro só como `.deb`, sem login. O
//! Vega baixa a versão conferida pelo Lyra, confere tamanho e SHA-256 com a
//! conferência do download guiado (o SHA-512 que o Serpro publica foi
//! conferido ao fixar a versão) e extrai do pacote só o `.jar` e o ícone em
//! `~/.local/opt/<pasta>`, sem root. Do pacote oficial ficam de fora:
//!
//! - o Java 7 embutido: o assinador roda com o Java 21 do Leap, que recebe
//!   atualizações de segurança pelo zypper;
//! - o `SerproSigner.sh`, que roda `xhost +local:all` e libera o servidor
//!   gráfico para qualquer processo local;
//! - o início automático no login (`/etc/xdg/autostart`): o assinador abre
//!   pelo menu, quando o usuário precisa.
//!
//! Na primeira vez que abre, o próprio assinador cria `~/.serpro` (configuração
//! e a autoridade certificadora local que os sites usam para falar com ele em
//! `localhost`) e instala essa autoridade nos perfis do Firefox. A remoção
//! apaga o programa e o atalho e deixa `~/.serpro`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use gettextrs::gettext;
use tauri::{AppHandle, Emitter};

use super::guided_download::{self, Expected};
use super::software::TransactionEvent;

const JAVA_HOME: &str = "/usr/lib64/jvm/jre-21-openjdk";

#[derive(Debug, Clone, Copy)]
pub struct Signer {
    pub id: &'static str,
    pub name: &'static str,
    /// Pasta em `~/.local/opt`.
    pub dir: &'static str,
    pub url: &'static str,
    pub expected: Expected,
    /// Pasta do programa dentro do `data.tar` do `.deb`.
    pub deb_root: &'static str,
    /// Arquivos copiados da pasta do programa, relativos a `deb_root`.
    pub files: &'static [&'static str],
    /// Classpath e classe principal.
    pub jar: &'static str,
    pub main: &'static str,
    pub icon: &'static str,
    pub comment: &'static str,
}

pub const SIGNERS: &[Signer] = &[Signer {
    id: "assinador-serpro",
    name: "Assinador Serpro",
    dir: "assinador-serpro",
    url: "https://www.serpro.gov.br/links-fixos-superiores/assinador-digital/assinador-serpro/arquivos/assinador-serpro_2-7-0_amd64.deb/@@download/file",
    expected: Expected {
        page: "https://www.serpro.gov.br/links-fixos-superiores/assinador-digital/assinador-serpro",
        file: "assinador-serpro_2.7.0_amd64.deb",
        size: 66_734_920,
        sha256: "1345dec44018ce92c10a89b8e2dff6696d021c271c3596160aa9c4903a166a90",
        version: "2.7.0",
    },
    deb_root: "./opt/serpro/tool/serpro-signer",
    files: &["lib/serpro-signer-2.7.0.jar", "serpro-signer.png"],
    jar: "lib/serpro-signer-2.7.0.jar",
    main: "org.demoiselle.signer.serpro.desktop.Main",
    icon: "serpro-signer.png",
    comment: "Assina e valida documentos no padrão ICP-Brasil",
}];

pub fn signer(id: &str) -> Option<&'static Signer> {
    SIGNERS.iter().find(|signer| signer.id == id)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn data_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"))
}

fn cache_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".cache"))
        .join("vega")
}

pub fn install_dir(home: &Path, signer: &Signer) -> PathBuf {
    home.join(".local/opt").join(signer.dir)
}

fn desktop_path(home: &Path, signer: &Signer) -> PathBuf {
    data_dir(home)
        .join("applications")
        .join(format!("lyra-{}.desktop", signer.id))
}

/// Script que abre o assinador com o Java do Leap, sem `xhost`.
pub fn launcher(signer: &Signer, home: &Path) -> String {
    let dir = install_dir(home, signer);
    format!(
        "#!/bin/sh\n# Gerado pelo Vega (vega#214).\nJAVA=\"${{JAVA_HOME:-{JAVA_HOME}}}/bin/java\"\ncd \"{dir}\" || exit 1\nexec \"$JAVA\" -Xms256m -Xmx1024m -Djava.net.preferIPv4Stack=true -Djava.net.useSystemProxies=true -Dfile.encoding=UTF-8 -cp \"{dir}/{jar}\" {main}\n",
        dir = dir.display(),
        jar = signer.jar,
        main = signer.main,
    )
}

pub fn desktop_entry(signer: &Signer, home: &Path) -> String {
    let dir = install_dir(home, signer);
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\nExec=\"{script}\"\nIcon={icon}\nTerminal=false\nCategories=Office;Security;\nX-Lyra-Vega=signer\n",
        name = signer.name,
        comment = signer.comment,
        script = dir.join(signer.id).display(),
        icon = dir.join(signer.icon).display(),
    )
}

fn write_file(path: &Path, text: &str, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let _ = std::fs::remove_file(path);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

/// Membro `data.tar.*` de um `.deb` (formato `ar`). Devolve o nome e o
/// conteúdo.
pub fn deb_data(deb: &[u8]) -> Option<(String, &[u8])> {
    let mut rest = deb.strip_prefix(b"!<arch>\n")?;
    while rest.len() >= 60 {
        let (header, body) = rest.split_at(60);
        let name = std::str::from_utf8(&header[..16]).ok()?.trim_end();
        let name = name.trim_end_matches('/');
        let size: usize = std::str::from_utf8(&header[48..58])
            .ok()?
            .trim()
            .parse()
            .ok()?;
        if body.len() < size {
            return None;
        }
        if name.starts_with("data.tar") {
            return Some((name.to_owned(), &body[..size]));
        }
        // Membros começam em posição par.
        rest = &body[(size + size % 2).min(body.len())..];
    }
    None
}

fn extract(signer: &Signer, deb: &Path, cache: &Path, target: &Path) -> Result<(), String> {
    let mut bytes = Vec::new();
    std::fs::File::open(deb)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| error.to_string())?;
    let (name, data) =
        deb_data(&bytes).ok_or_else(|| gettext("O pacote baixado não é um .deb válido."))?;
    let tarball = cache.join(&name);
    std::fs::write(&tarball, data).map_err(|error| error.to_string())?;
    let staging = cache.join(format!("{}.extract", signer.id));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let mut command = std::process::Command::new("tar");
    command.arg("-xf").arg(&tarball).arg("-C").arg(&staging);
    for file in signer.files {
        command.arg(format!("{}/{file}", signer.deb_root));
    }
    crate::webkit_env::clean_command(&mut command);
    let status = command.status().map_err(|error| error.to_string());
    let _ = std::fs::remove_file(&tarball);
    let result = status.and_then(|status| {
        if !status.success() {
            return Err(gettext("Não foi possível extrair {file} ({status}).")
                .replace("{file}", signer.expected.file)
                .replace("{status}", &status.to_string()));
        }
        let root = staging.join(signer.deb_root.trim_start_matches("./"));
        for file in signer.files {
            let to = target.join(file);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::copy(root.join(file), &to).map_err(|error| error.to_string())?;
        }
        Ok(())
    });
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn install(signer: &Signer, report: &dyn Fn(u32, String)) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let cache = cache_dir(&home);
    std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let deb = cache.join(signer.expected.file);
    let dir = install_dir(&home, signer);
    let label =
        gettext("Baixando {file} do site do Serpro…").replace("{file}", signer.expected.file);
    report(5, label.clone());
    let result = super::receita_apps::download_reporting(signer.url, &deb, |done, total| {
        let (percent, message) = super::receita_apps::download_step(&label, done, total, 5, 70);
        report(percent, message);
    })
    .and_then(|()| {
        report(70, gettext("Conferindo o pacote…"));
        guided_download::verify(&signer.expected, &deb)
            .map_err(|rejection| rejection.message(&signer.expected))
    })
    .and_then(|()| {
        report(
            80,
            gettext("Instalando {name}…").replace("{name}", signer.name),
        );
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
        }
        extract(signer, &deb, &cache, &dir)
    });
    let _ = std::fs::remove_file(&deb);
    result?;
    write_file(&dir.join(signer.id), &launcher(signer, &home), 0o755)?;
    write_file(
        &desktop_path(&home, signer),
        &desktop_entry(signer, &home),
        0o644,
    )
}

fn remove(signer: &Signer) -> Result<(), String> {
    let home = home().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let _ = std::fs::remove_file(desktop_path(&home, signer));
    let dir = install_dir(&home, signer);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn refresh_menu(home: &Path) {
    let mut command = std::process::Command::new("update-desktop-database");
    command.arg(data_dir(home).join("applications"));
    crate::webkit_env::clean_command(&mut command);
    let _ = command.status();
}

#[tauri::command]
pub fn signers_installed() -> Vec<String> {
    let Some(home) = home() else {
        return Vec::new();
    };
    SIGNERS
        .iter()
        .filter(|signer| install_dir(&home, signer).join(signer.jar).is_file())
        .map(|signer| signer.id.to_owned())
        .collect()
}

fn spawn(app: AppHandle, signer: &'static Signer, install_it: bool) -> u32 {
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let report = |percent: u32, message: String| {
            let _ = app.emit(
                "software-transaction",
                TransactionEvent::Progress {
                    id: tx,
                    percent,
                    message,
                },
            );
        };
        let result = if install_it {
            install(signer, &report)
        } else {
            remove(signer)
        };
        if let Some(home) = home() {
            refresh_menu(&home);
        }
        let _ = app.emit(
            "software-transaction",
            TransactionEvent::Finished {
                id: tx,
                success: result.is_ok(),
                message: result.err().unwrap_or_else(|| gettext("Concluído")),
            },
        );
    });
    tx
}

#[tauri::command]
pub fn signer_install(app: AppHandle, id: String) -> Result<u32, String> {
    let signer = signer(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    Ok(spawn(app, signer, true))
}

#[tauri::command]
pub fn signer_remove(app: AppHandle, id: String) -> Result<u32, String> {
    let signer = signer(&id).ok_or_else(|| gettext("Aplicativo desconhecido."))?;
    Ok(spawn(app, signer, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
            0,
            0,
            0,
            100644,
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(b'\n');
        }
        out
    }

    #[test]
    fn deb_data_finds_the_data_member() {
        let mut deb = b"!<arch>\n".to_vec();
        deb.extend(member("debian-binary", b"2.0\n"));
        deb.extend(member("control.tar.gz", b"abc"));
        deb.extend(member("data.tar.gz", b"payload"));
        let (name, data) = deb_data(&deb).unwrap();
        assert_eq!(name, "data.tar.gz");
        assert_eq!(data, b"payload");
        assert!(deb_data(b"not a deb").is_none());
    }

    #[test]
    fn signers_are_pinned_to_the_official_site() {
        for signer in SIGNERS {
            assert!(signer.url.starts_with("https://www.serpro.gov.br/"));
            assert_eq!(signer.expected.sha256.len(), 64);
            assert!(signer.files.contains(&signer.jar));
            assert!(signer.files.contains(&signer.icon));
        }
    }

    #[test]
    fn launcher_uses_the_leap_java_without_xhost() {
        let serpro = signer("assinador-serpro").unwrap();
        let script = launcher(serpro, Path::new("/home/u"));
        assert!(script.contains("JAVA_HOME:-/usr/lib64/jvm/jre-21-openjdk"));
        assert!(script.contains("/home/u/.local/opt/assinador-serpro/lib/serpro-signer-2.7.0.jar"));
        assert!(!script.contains("xhost"));
        let entry = desktop_entry(serpro, Path::new("/home/u"));
        assert!(entry.contains("Exec=\"/home/u/.local/opt/assinador-serpro/assinador-serpro\""));
        assert!(!entry.contains("Autostart"));
    }
}
