use super::{Connection, Result, data_directory};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn create_shortcut(
    connection: Connection,
    uuid: &str,
    name: &str,
    xdg: Option<&Path>,
    home: &Path,
) -> Result<PathBuf> {
    let uuid = uuid::Uuid::parse_str(uuid).map_err(|e| e.to_string())?;
    let directory = data_directory(xdg, home)?
        .parent()
        .ok_or("Invalid data directory")?
        .join("applications");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let scope = match connection {
        Connection::Personal => "personal",
        Connection::System => "system",
    };
    let target = directory.join(format!("org.lyraos.VMs.{scope}.{uuid}.desktop"));
    let name = name
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Lyra VMs — {name}\nExec=/usr/bin/lyra-vms --connect {} --uuid {uuid}\nIcon=computer-symbolic\nTerminal=false\nCategories=System;Emulator;\n",
        connection.uri()
    );
    // Publish a complete file atomically without replacing customized entries.
    let temporary = directory.join(format!(".lyra-vms-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(entry.as_bytes())?;
        file.sync_all()?;
        std::fs::hard_link(&temporary, &target)
    })();
    let _ = std::fs::remove_file(&temporary);
    result.map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            "A shortcut for this machine already exists.".into()
        } else {
            e.to_string()
        }
    })?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcuts_escape_names_and_never_replace_existing_entries() {
        let root = std::env::temp_dir().join(format!("lyra-shortcut-{}", uuid::Uuid::new_v4()));
        let id = "a84fdf47-f7f3-4306-9358-d2d3f38af37c";
        let path = create_shortcut(
            Connection::Personal,
            id,
            "Test\nExec=unexpected",
            Some(&root),
            Path::new("/unused"),
        )
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content.lines().filter(|s| s.starts_with("Exec=")).count(),
            1
        );
        assert!(content.contains("Name=Lyra VMs — Test\\nExec=unexpected\n"));
        assert!(
            create_shortcut(
                Connection::Personal,
                id,
                "Changed",
                Some(&root),
                Path::new("/unused")
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root.join("applications")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
