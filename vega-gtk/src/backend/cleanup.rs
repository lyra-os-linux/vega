//! Liberar espaço (add-space-cleanup). As categorias do sistema passam pelo
//! vegad (`org.lyraos.Vega1.Cleanup`); as da pasta pessoal — lixeira,
//! miniaturas, `~/.cache` e runtimes Flatpak do usuário — são medidas e
//! limpas aqui, na sessão, sem senha: o root não precisa apagar nada na pasta
//! do usuário.
//!
//! Nada aqui segue links simbólicos: um link dentro de uma categoria é
//! apagado como link, e o destino fica intacto.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gettextrs::gettext;
use lyra_vega_dbus::{CleanupCategory, CleanupClient, CleanupEvent, CleanupEventStream};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::{CmdResult, State, err};

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    id: String,
    bytes: u64,
    can_clean: bool,
    count: Option<u64>,
    size_unknown: bool,
    minimum: bool,
    reason: Option<String>,
    items: Vec<String>,
}

impl From<CleanupCategory> for Category {
    fn from(category: CleanupCategory) -> Self {
        Self {
            id: category.id,
            bytes: category.bytes,
            can_clean: category.can_clean,
            count: category.detail.count,
            size_unknown: category.detail.size_unknown,
            minimum: category.detail.minimum,
            reason: category.detail.reason,
            items: category.detail.items,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Event {
    Progress {
        job: u32,
        id: String,
        percent: u32,
    },
    Category {
        job: u32,
        id: String,
        result: String,
        freed: u64,
    },
    Finished {
        job: u32,
        result: String,
        freed: u64,
        message: String,
    },
}

#[tauri::command]
pub async fn cleanup_scan(state: State<'_>) -> CmdResult<Vec<Category>> {
    state
        .dbus()
        .await?
        .cleanup()
        .scan()
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(err)
}

/// Assina os sinais antes de limpar, para não perder uma categoria rápida.
#[tauri::command]
pub async fn cleanup_start(app: AppHandle, state: State<'_>, ids: Vec<String>) -> CmdResult<u32> {
    let cleanup = state.dbus().await?.cleanup();
    let events = cleanup.subscribe().await.map_err(err)?;
    let job = cleanup.clean(&ids).await.map_err(err)?;
    tauri::async_runtime::spawn(follow(app, events, job));
    Ok(job)
}

async fn follow(app: AppHandle, mut events: CleanupEventStream, job: u32) {
    loop {
        let event = match events.next().await {
            Ok(CleanupEvent::Progress(p)) if p.job_id == job => Event::Progress {
                job,
                id: p.id,
                percent: p.percent,
            },
            Ok(CleanupEvent::CategoryFinished(c)) if c.job_id == job => Event::Category {
                job,
                id: c.id,
                result: c.result,
                freed: c.freed_bytes,
            },
            Ok(CleanupEvent::Finished(f)) if f.job_id == job => Event::Finished {
                job,
                result: f.result,
                freed: f.freed_bytes,
                message: String::new(),
            },
            Ok(_) => continue,
            Err(error) => Event::Finished {
                job,
                result: "failed".into(),
                freed: 0,
                message: err(error),
            },
        };
        let done = matches!(event, Event::Finished { .. });
        let _ = app.emit("cleanup-event", event);
        if done {
            break;
        }
    }
}

// ---------- Categorias do usuário ----------

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UserEntry {
    name: String,
    bytes: u64,
    /// Programas com arquivos abertos na pasta: apagá-la com eles rodando
    /// os quebra (o localsearch, com `tracker3` apagado, em 30/10/2026).
    in_use_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UserCategory {
    id: String,
    bytes: u64,
    count: u64,
    /// Subpastas de `~/.cache`, por aplicativo, da maior para a menor.
    entries: Vec<UserEntry>,
    /// Refs de runtimes Flatpak sem uso.
    items: Vec<String>,
    available: bool,
}

struct Home {
    data: PathBuf,
    cache: PathBuf,
    /// `/proc`, trocado nos testes.
    proc: PathBuf,
}

impl Home {
    fn from_env() -> Option<Self> {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        let xdg = |var: &str, fallback: &str| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.join(fallback))
        };
        Some(Self {
            data: xdg("XDG_DATA_HOME", ".local/share"),
            cache: xdg("XDG_CACHE_HOME", ".cache"),
            proc: PathBuf::from("/proc"),
        })
    }

    fn trash(&self) -> PathBuf {
        self.data.join("Trash")
    }

    fn thumbnails(&self) -> PathBuf {
        self.cache.join("thumbnails")
    }
}

/// Soma o espaço ocupado sem seguir links, sem sair do sistema de arquivos e
/// contando hard links uma vez.
fn tree_size(path: &Path, seen: &mut HashSet<(u64, u64)>) -> (u64, u64) {
    let Ok(root) = fs::symlink_metadata(path) else {
        return (0, 0);
    };
    let device = root.dev();
    let mut stack = vec![(path.to_path_buf(), root)];
    let (mut bytes, mut files) = (0, 0);
    while let Some((path, meta)) = stack.pop() {
        if meta.dev() != device {
            continue;
        }
        if meta.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Ok(child) = entry.metadata() {
                        stack.push((entry.path(), child));
                    }
                }
            }
            continue;
        }
        if meta.nlink() > 1 && !seen.insert((meta.dev(), meta.ino())) {
            continue;
        }
        bytes += meta.blocks() * 512;
        files += 1;
    }
    (bytes, files)
}

/// Apaga uma entrada sem seguir links: um link sai como link, e
/// `remove_dir_all` não atravessa links nos níveis de baixo.
fn remove_entry(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Esvazia `dir` sem apagar a própria pasta. Recusa se ela for um link.
fn empty_dir(dir: &Path) -> io::Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(io::Error::other(format!(
                "{} é um link simbólico",
                dir.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    let mut first_error = None;
    for entry in fs::read_dir(dir)?.flatten() {
        if let Err(error) = remove_entry(&entry.path()) {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

fn measure_dirs(dirs: &[PathBuf]) -> (u64, u64) {
    let mut seen = HashSet::new();
    dirs.iter()
        .filter(|dir| !is_link(dir))
        .map(|dir| {
            let (bytes, files) = tree_size(dir, &mut seen);
            // A raiz conta como arquivo quando não é pasta; aqui só interessa o conteúdo.
            (bytes, files)
        })
        .fold((0, 0), |(b, f), (bytes, files)| (b + bytes, f + files))
}

fn trash_dirs(home: &Home) -> Vec<PathBuf> {
    ["files", "info", "expunged"]
        .iter()
        .map(|name| home.trash().join(name))
        .collect()
}

/// Para cada subpasta de `~/.cache`, os programas do usuário que têm um
/// arquivo aberto nela ou estão dentro dela (`/proc/<pid>/fd` e `cwd`). Só
/// os processos do próprio usuário são legíveis, e são os que importam.
fn cache_users(home: &Home) -> BTreeMap<String, BTreeSet<String>> {
    let mut users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let Ok(processes) = fs::read_dir(&home.proc) else {
        return users;
    };
    for process in processes.flatten() {
        let pid = process.file_name().to_string_lossy().into_owned();
        if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let dir = process.path();
        let mut targets: Vec<PathBuf> = fs::read_dir(dir.join("fd"))
            .map(|fds| {
                fds.flatten()
                    .filter_map(|fd| fs::read_link(fd.path()).ok())
                    .collect()
            })
            .unwrap_or_default();
        targets.extend(fs::read_link(dir.join("cwd")).ok());
        let names: BTreeSet<String> = targets
            .iter()
            .filter_map(|target| target.strip_prefix(&home.cache).ok())
            .filter_map(|rest| rest.components().next())
            .map(|first| first.as_os_str().to_string_lossy().into_owned())
            .collect();
        if names.is_empty() {
            continue;
        }
        let comm = fs::read_to_string(dir.join("comm"))
            .map(|comm| comm.trim().to_owned())
            .unwrap_or(pid);
        for name in names {
            users.entry(name).or_default().insert(comm.clone());
        }
    }
    users
}

/// Subpastas de `~/.cache`, sem as miniaturas, que têm a própria categoria.
fn cache_entries(home: &Home) -> Vec<UserEntry> {
    if is_link(&home.cache) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(&home.cache) else {
        return Vec::new();
    };
    let users = cache_users(home);
    let mut seen = HashSet::new();
    let mut list: Vec<UserEntry> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name == "thumbnails" {
                return None;
            }
            let path = entry.path();
            let bytes = if is_link(&path) {
                0
            } else {
                tree_size(&path, &mut seen).0
            };
            let in_use_by = users
                .get(&name)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default();
            Some(UserEntry {
                name,
                bytes,
                in_use_by,
            })
        })
        .collect();
    list.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    list
}

fn flatpak(args: &[&str]) -> Command {
    let mut command = Command::new("flatpak");
    command.args(args).env("LC_ALL", "C");
    crate::webkit_env::clean_command(&mut command);
    command
}

/// Linha da tabela que `flatpak uninstall --unused` mostra antes de
/// perguntar: ` 1.\t   \torg.gtk.Gtk3theme.Adwaita-dark\t3.22\tr`.
pub fn parse_flatpak_unused(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let number = fields.next()?;
            if !number.ends_with('.')
                || !number[..number.len() - 1]
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            {
                return None;
            }
            let (id, branch, op) = (fields.next()?, fields.next()?, fields.next()?);
            (op == "r" && fields.next().is_none() && id.contains('.'))
                .then(|| (id.to_owned(), branch.to_owned()))
        })
        .collect()
}

/// Pergunta ao próprio Flatpak o que ele removeria e responde "n".
fn unused_user_runtimes() -> Option<Vec<(String, String)>> {
    let mut child = flatpak(&["uninstall", "--unused", "--user"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"n\n");
    }
    let output = child.wait_with_output().ok()?;
    Some(parse_flatpak_unused(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn user_flatpak_category(home: &Home) -> UserCategory {
    let Some(refs) = unused_user_runtimes() else {
        return UserCategory {
            id: "user-flatpak-unused".into(),
            bytes: 0,
            count: 0,
            entries: Vec::new(),
            items: Vec::new(),
            available: false,
        };
    };
    let runtime = home.data.join("flatpak").join("runtime");
    let mut dirs = Vec::new();
    for (id, branch) in &refs {
        if let Ok(arches) = fs::read_dir(runtime.join(id)) {
            dirs.extend(arches.flatten().map(|arch| arch.path().join(branch)));
        }
    }
    UserCategory {
        id: "user-flatpak-unused".into(),
        bytes: measure_dirs(&dirs).0,
        count: refs.len() as u64,
        entries: Vec::new(),
        items: refs
            .iter()
            .map(|(id, branch)| format!("{id}//{branch}"))
            .collect(),
        available: true,
    }
}

fn scan_user(home: &Home, with_flatpak: bool) -> Vec<UserCategory> {
    let (trash_bytes, trash_files) = measure_dirs(&trash_dirs(home));
    let (thumb_bytes, thumb_files) = measure_dirs(&[home.thumbnails()]);
    let entries = cache_entries(home);
    let mut categories = vec![
        UserCategory {
            id: "trash".into(),
            bytes: trash_bytes,
            count: trash_files,
            entries: Vec::new(),
            items: Vec::new(),
            available: true,
        },
        UserCategory {
            id: "thumbnails".into(),
            bytes: thumb_bytes,
            count: thumb_files,
            entries: Vec::new(),
            items: Vec::new(),
            available: true,
        },
        UserCategory {
            id: "user-cache".into(),
            bytes: entries.iter().map(|entry| entry.bytes).sum(),
            count: entries.len() as u64,
            entries,
            items: Vec::new(),
            available: true,
        },
    ];
    if with_flatpak {
        categories.push(user_flatpak_category(home));
    }
    categories
}

/// Um nome de subpasta de `~/.cache`: um componente só, nada de `..`.
fn valid_entry(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && name != "thumbnails"
}

fn clean_user(home: &Home, id: &str, entries: &[String]) -> Result<(), String> {
    let result = match id {
        "trash" => trash_dirs(home).iter().try_for_each(|dir| empty_dir(dir)),
        "thumbnails" => empty_dir(&home.thumbnails()),
        "user-cache" => {
            if entries.is_empty() || !entries.iter().all(|name| valid_entry(name)) {
                return Err(gettext("Escolha as pastas do cache que serão apagadas."));
            }
            if is_link(&home.cache) {
                return Err(gettext(
                    "A pasta do cache é um link simbólico; nada foi apagado.",
                ));
            }
            // Conferido de novo na hora: o programa pode ter aberto depois da lista.
            let users = cache_users(home);
            let busy: Vec<String> = entries
                .iter()
                .filter_map(|name| {
                    let set = users.get(name)?;
                    Some(format!(
                        "{name} ({})",
                        set.iter().cloned().collect::<Vec<_>>().join(", ")
                    ))
                })
                .collect();
            if !busy.is_empty() {
                return Err(gettext("Em uso; feche o programa e tente de novo: {list}")
                    .replace("{list}", &busy.join("; ")));
            }
            entries
                .iter()
                .try_for_each(|name| match remove_entry(&home.cache.join(name)) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                    other => other,
                })
        }
        "user-flatpak-unused" => {
            let status = flatpak(&["uninstall", "--unused", "--user", "--noninteractive", "-y"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(err)?;
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!("flatpak: {status}")))
            }
        }
        _ => return Err(gettext("Categoria de limpeza desconhecida.")),
    };
    result.map_err(err)
}

fn category_bytes(categories: &[UserCategory], id: &str, entries: &[String]) -> u64 {
    categories
        .iter()
        .find(|category| category.id == id)
        .map(|category| {
            if id == "user-cache" {
                category
                    .entries
                    .iter()
                    .filter(|entry| entries.contains(&entry.name))
                    .map(|entry| entry.bytes)
                    .sum()
            } else {
                category.bytes
            }
        })
        .unwrap_or(0)
}

#[tauri::command]
pub async fn cleanup_user_scan() -> CmdResult<Vec<UserCategory>> {
    tauri::async_runtime::spawn_blocking(|| {
        let home = Home::from_env().ok_or_else(|| gettext("HOME não definido."))?;
        Ok(scan_user(&home, true))
    })
    .await
    .map_err(err)?
}

/// Limpa uma categoria do usuário e devolve os bytes liberados (medidos
/// antes e depois).
#[tauri::command]
pub async fn cleanup_user_clean(id: String, entries: Vec<String>) -> CmdResult<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = Home::from_env().ok_or_else(|| gettext("HOME não definido."))?;
        let flatpak = id == "user-flatpak-unused";
        let before = category_bytes(&scan_user(&home, flatpak), &id, &entries);
        clean_user(&home, &id, &entries)?;
        let after = category_bytes(&scan_user(&home, flatpak), &id, &entries);
        Ok(before.saturating_sub(after))
    })
    .await
    .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pasta temporária apagada no fim do teste.
    struct TempDir(PathBuf);

    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn home() -> (TempDir, Home) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = TempDir(std::env::temp_dir().join(format!(
            "vega-cleanup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir_all(dir.path()).unwrap();
        let home = Home {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
            proc: dir.path().join("proc"),
        };
        (dir, home)
    }

    fn write(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; size]).unwrap();
    }

    #[test]
    fn flatpak_table_is_parsed_from_the_real_prompt() {
        let out = "\n\n 1.\t   \torg.gtk.Gtk3theme.Adwaita-dark\t3.22\tr\n\nProceed with these changes to the user installation? [Y/n]: n\n";
        assert_eq!(
            parse_flatpak_unused(out),
            [(
                "org.gtk.Gtk3theme.Adwaita-dark".to_owned(),
                "3.22".to_owned()
            )]
        );
        assert!(parse_flatpak_unused("Nothing unused to uninstall\n").is_empty());
    }

    #[test]
    fn cache_lists_each_app_and_cleaning_one_keeps_the_others() {
        let (_dir, home) = home();
        write(&home.cache.join("debuginfod_client/a/debuginfo"), 64 << 10);
        write(&home.cache.join("mozilla/firefox/cache2/entry"), 16 << 10);
        write(&home.thumbnails().join("large/x.png"), 4 << 10);
        let entries = cache_entries(&home);
        assert_eq!(entries[0].name, "debuginfod_client");
        assert_eq!(
            entries.len(),
            2,
            "as miniaturas têm categoria própria: {entries:?}"
        );

        clean_user(&home, "user-cache", &["debuginfod_client".into()]).unwrap();
        assert!(!home.cache.join("debuginfod_client").exists());
        assert!(home.cache.join("mozilla/firefox/cache2/entry").exists());
        assert!(home.thumbnails().join("large/x.png").exists());
    }

    #[test]
    fn a_link_in_the_cache_is_removed_and_its_target_kept() {
        let (dir, home) = home();
        let documents = dir.path().join("Documentos");
        write(&documents.join("tese.odt"), 100);
        fs::create_dir_all(&home.cache).unwrap();
        std::os::unix::fs::symlink(&documents, home.cache.join("app")).unwrap();
        let entries = cache_entries(&home);
        assert_eq!(
            entries,
            [UserEntry {
                name: "app".into(),
                bytes: 0,
                in_use_by: Vec::new()
            }]
        );

        clean_user(&home, "user-cache", &["app".into()]).unwrap();
        assert!(!is_link(&home.cache.join("app")));
        assert!(documents.join("tese.odt").exists());
    }

    #[test]
    fn a_link_deeper_in_the_trash_is_not_followed() {
        let (dir, home) = home();
        let documents = dir.path().join("Documentos");
        write(&documents.join("tese.odt"), 100);
        write(&home.trash().join("files/old/report.pdf"), 8 << 10);
        write(&home.trash().join("info/old.trashinfo"), 100);
        std::os::unix::fs::symlink(&documents, home.trash().join("files/old/docs")).unwrap();
        let before = scan_user(&home, false);
        assert!(before[0].bytes >= 8 << 10);

        clean_user(&home, "trash", &[]).unwrap();
        assert_eq!(fs::read_dir(home.trash().join("files")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(home.trash().join("info")).unwrap().count(), 0);
        assert!(documents.join("tese.odt").exists());
        assert_eq!(scan_user(&home, false)[0].bytes, 0);
    }

    #[test]
    fn a_cache_folder_open_by_a_program_is_marked_and_refused() {
        let (dir, home) = home();
        write(&home.cache.join("tracker3/files/meta.db"), 4 << 10);
        write(&home.cache.join("mozilla/cache2/entry"), 4 << 10);
        // Um localsearch falso com o banco aberto.
        let process = dir.path().join("proc/4242");
        fs::create_dir_all(process.join("fd")).unwrap();
        fs::write(process.join("comm"), "localsearch-3\n").unwrap();
        std::os::unix::fs::symlink(
            home.cache.join("tracker3/files/meta.db"),
            process.join("fd/7"),
        )
        .unwrap();
        std::os::unix::fs::symlink("/dev/null", process.join("fd/0")).unwrap();

        let entries = cache_entries(&home);
        let tracker = entries.iter().find(|e| e.name == "tracker3").unwrap();
        let mozilla = entries.iter().find(|e| e.name == "mozilla").unwrap();
        assert_eq!(tracker.in_use_by, ["localsearch-3"]);
        assert!(mozilla.in_use_by.is_empty());

        let error =
            clean_user(&home, "user-cache", &["tracker3".into(), "mozilla".into()]).unwrap_err();
        assert!(error.contains("tracker3 (localsearch-3)"), "{error}");
        assert!(home.cache.join("tracker3/files/meta.db").exists());
        assert!(
            home.cache.join("mozilla/cache2/entry").exists(),
            "nada sai se algo está em uso"
        );
        clean_user(&home, "user-cache", &["mozilla".into()]).unwrap();
        assert!(!home.cache.join("mozilla").exists());
    }

    #[test]
    fn cache_entries_must_be_single_names() {
        let (_dir, home) = home();
        for bad in ["", "..", "../Documentos", "a/b", "thumbnails"] {
            assert!(
                clean_user(&home, "user-cache", &[bad.into()]).is_err(),
                "{bad}"
            );
        }
        assert!(clean_user(&home, "user-cache", &[]).is_err());
        assert!(clean_user(&home, "Documentos", &[]).is_err());
    }

    #[test]
    fn a_linked_category_root_is_refused() {
        let (dir, home) = home();
        let elsewhere = dir.path().join("elsewhere");
        write(&elsewhere.join("keep"), 10);
        fs::create_dir_all(&home.cache).unwrap();
        std::os::unix::fs::symlink(&elsewhere, home.thumbnails()).unwrap();
        assert!(clean_user(&home, "thumbnails", &[]).is_err());
        assert!(elsewhere.join("keep").exists());
    }
}
