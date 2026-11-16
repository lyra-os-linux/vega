//! Serviços em contêiner (Vega → Contêineres, vega#186, vega#190 e vega#284).
//!
//! O Oracle AI Database Free e o Microsoft SQL Server (e, depois, o SAP de estudo) só são distribuídos
//! como imagem de contêiner. O Vega usa o Podman sem root do Leap 16.1, na
//! conta do usuário: nada roda como root e nada passa pelo vegad. Cada
//! serviço vira um Quadlet (`~/.config/containers/systemd/lyra-<id>.container`)
//! gerido pelo `systemd --user`, com volume nomeado para os dados. Não há
//! seção `[Install]`: o serviço não liga com o computador.
//!
//! Só os serviços de `SERVICES` são aceitos, com a imagem e a versão fixadas
//! aqui; a interface manda apenas o id. As portas ficam em 127.0.0.1. A senha
//! de administrador do banco é gerada pelo Vega e guardada num arquivo 0600
//! na conta do usuário (`~/.config/lyra-containers/<id>.env`).

use std::path::{Path, PathBuf};
use std::process::Command;

use gettextrs::gettext;
use serde::Serialize;
use tauri::AppHandle;

use super::flatpak_session::follow_then;

#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub id: &'static str,
    pub name: &'static str,
    pub image: &'static str,
    /// (porta em 127.0.0.1, porta no contêiner).
    pub ports: &'static [(u16, u16)],
    /// Onde o volume de dados é montado no contêiner.
    pub data: &'static str,
    /// Variável de ambiente que recebe a senha gerada.
    pub password_env: &'static str,
    pub min_ram_mib: u64,
    pub min_disk_gib: u64,
    /// Texto de conexão mostrado na interface (`{password}` não entra aqui).
    pub connection: &'static str,
    /// Usuários que recebem a senha gerada.
    pub users: &'static str,
    /// Comando para abrir o cliente de linha de comando dentro do contêiner.
    pub terminal: &'static str,
    /// Variáveis fixas, sem segredo (vão no Quadlet).
    pub environment: &'static [&'static str],
    /// Verificação de saúde, quando a imagem não traz uma.
    pub health: Option<&'static str>,
    /// Licença que o usuário precisa aceitar antes de instalar.
    pub license: Option<License>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct License {
    /// Página oficial com os termos.
    pub url: &'static str,
    /// Variável que registra o aceite; só entra no Quadlet depois dele.
    #[serde(skip)]
    pub accept_env: &'static str,
}

pub const SERVICES: &[Service] = &[
    Service {
        id: "oracle-free",
        name: "Oracle AI Database 26ai Free",
        image: "container-registry.oracle.com/database/free:23.26.3.0",
        ports: &[(1521, 1521)],
        data: "/opt/oracle/oradata",
        password_env: "ORACLE_PWD",
        min_ram_mib: 2048,
        min_disk_gib: 16,
        connection: "localhost:1521/FREEPDB1",
        users: "SYS, SYSTEM, PDBADMIN",
        terminal: "podman exec -it lyra-oracle-free sqlplus system@FREEPDB1",
        environment: &[],
        health: None,
        license: None,
    },
    // vega#284: só a imagem oficial; a instalação nativa do mssql-server
    // não é suportada no Leap.
    Service {
        id: "mssql",
        name: "Microsoft SQL Server 2022 Developer",
        image: "mcr.microsoft.com/mssql/server:2022-CU28-ubuntu-22.04",
        ports: &[(1433, 1433)],
        data: "/var/opt/mssql",
        password_env: "MSSQL_SA_PASSWORD",
        min_ram_mib: 2048,
        min_disk_gib: 10,
        connection: "localhost,1433",
        users: "sa",
        terminal: "podman exec -it lyra-mssql /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -C",
        environment: &["MSSQL_PID=Developer"],
        // A imagem não traz HEALTHCHECK: pronto quando aceita a conexão. O
        // `$$` chega ao contêiner como `$` (o systemd expande o simples).
        health: Some(
            "/opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -P \"$$MSSQL_SA_PASSWORD\" -C -b -Q \"SELECT 1\"",
        ),
        license: Some(License {
            url: "https://go.microsoft.com/fwlink/?linkid=857698",
            accept_env: "ACCEPT_EULA=Y",
        }),
    },
];

pub fn service(id: &str) -> Option<&'static Service> {
    SERVICES.iter().find(|service| service.id == id)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".config")))
}

pub fn unit(service: &Service) -> String {
    format!("lyra-{}", service.id)
}

pub fn volume(service: &Service) -> String {
    format!("lyra-{}-data", service.id)
}

fn quadlet_path(config: &Path, service: &Service) -> PathBuf {
    config
        .join("containers/systemd")
        .join(format!("{}.container", unit(service)))
}

fn env_path(config: &Path, service: &Service) -> PathBuf {
    config
        .join("lyra-containers")
        .join(format!("{}.env", service.id))
}

/// Quadlet do serviço. Sem `[Install]`: não liga junto com o computador.
pub fn quadlet(service: &Service, env_file: &Path) -> String {
    let mut text = format!(
        "# Gerado pelo Vega (vega#186). Remova pelo Vega → Contêineres.\n\
         [Unit]\n\
         Description={name} (Vega)\n\n\
         [Container]\n\
         ContainerName={unit}\n\
         Image={image}\n\
         Volume={volume}:{data}\n\
         EnvironmentFile={env}\n",
        name = service.name,
        unit = unit(service),
        image = service.image,
        volume = volume(service),
        data = service.data,
        env = env_file.display(),
    );
    for (host, container) in service.ports {
        text.push_str(&format!("PublishPort=127.0.0.1:{host}:{container}\n"));
    }
    for line in service.environment {
        text.push_str(&format!("Environment={line}\n"));
    }
    if let Some(license) = service.license {
        text.push_str(&format!("Environment={}\n", license.accept_env));
    }
    if let Some(health) = service.health {
        text.push_str(&format!(
            "HealthCmd={health}\nHealthInterval=15s\nHealthStartPeriod=60s\n"
        ));
    }
    // A primeira partida cria o banco e leva minutos.
    text.push_str("\n[Service]\nTimeoutStartSec=900\nTimeoutStopSec=300\n");
    text
}

/// Senha só com letras e números (o Oracle recusa alguns símbolos), com
/// maiúscula, minúscula e dígito garantidos.
fn random_password() -> Result<String, String> {
    let mut bytes = [0u8; 12];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut bytes))
        .map_err(|error| error.to_string())?;
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let body: String = bytes
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect();
    Ok(format!("Ly{body}7"))
}

fn write_file(path: &Path, text: &str, mode: u32) -> Result<(), String> {
    use std::io::Write;
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

fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    crate::webkit_env::clean_command(&mut command);
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn systemctl(args: &[&str]) -> bool {
    let mut all = vec!["--user"];
    all.extend_from_slice(args);
    run("systemctl", &all).is_some()
}

fn ram_mib() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| line.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .map(|kib| kib / 1024)
        .unwrap_or(0)
}

/// Espaço livre onde o Podman sem root guarda imagens e volumes.
fn free_disk_gib() -> u64 {
    let Some(home) = home() else { return 0 };
    run(
        "df",
        &["--output=avail", "-B1G", &home.display().to_string()],
    )
    .and_then(|text| text.lines().last()?.trim().parse().ok())
    .unwrap_or(0)
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub id: &'static str,
    pub name: &'static str,
    pub image: &'static str,
    pub podman: bool,
    pub installed: bool,
    /// active, activating, inactive, failed…
    pub state: String,
    /// healthy, starting, unhealthy ou vazio (sem contêiner).
    pub health: String,
    pub connection: &'static str,
    pub users: &'static str,
    pub terminal: &'static str,
    pub license: Option<License>,
    pub ram_mib: u64,
    pub min_ram_mib: u64,
    pub free_disk_gib: u64,
    pub min_disk_gib: u64,
}

#[tauri::command]
pub fn containers_list() -> Vec<Status> {
    let podman = run("podman", &["--version"]).is_some();
    let config = config_dir();
    let ram = ram_mib();
    let disk = free_disk_gib();
    SERVICES
        .iter()
        .map(|service| {
            let installed = config
                .as_ref()
                .is_some_and(|config| quadlet_path(config, service).is_file());
            let name = unit(service);
            let state = if installed {
                run("systemctl", &["--user", "is-active", &name])
                    .unwrap_or_else(|| "inactive".into())
            } else {
                String::new()
            };
            let health = if podman && state == "active" {
                run(
                    "podman",
                    &["inspect", "--format", "{{.State.Health.Status}}", &name],
                )
                .unwrap_or_default()
            } else {
                String::new()
            };
            Status {
                id: service.id,
                name: service.name,
                image: service.image,
                podman,
                installed,
                state,
                health,
                connection: service.connection,
                users: service.users,
                terminal: service.terminal,
                license: service.license,
                ram_mib: ram,
                min_ram_mib: service.min_ram_mib,
                free_disk_gib: disk,
                min_disk_gib: service.min_disk_gib,
            }
        })
        .collect()
}

/// Senha gerada para o serviço (para a interface mostrar ou copiar).
#[tauri::command]
pub fn containers_password(id: String) -> Result<String, String> {
    let service = service(&id).ok_or_else(|| gettext("Serviço desconhecido."))?;
    let config = config_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let text = std::fs::read_to_string(env_path(&config, service))
        .map_err(|_| gettext("O serviço ainda não foi instalado."))?;
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{}=", service.password_env)))
        .map(str::to_owned)
        .ok_or_else(|| gettext("Senha não encontrada."))
}

fn prepare(service: &Service) -> Result<(), String> {
    if ram_mib() < service.min_ram_mib {
        return Err(gettext(
            "Este computador não tem memória suficiente para o serviço.",
        ));
    }
    if free_disk_gib() < service.min_disk_gib {
        return Err(gettext(
            "Não há espaço livre suficiente na pasta pessoal para o serviço.",
        ));
    }
    let config = config_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let env = env_path(&config, service);
    // Reinstalar mantém a senha: o banco no volume já foi criado com ela.
    if !env.is_file() {
        write_file(
            &env,
            &format!("{}={}\n", service.password_env, random_password()?),
            0o600,
        )?;
    }
    write_file(
        &quadlet_path(&config, service),
        &quadlet(service, &env),
        0o644,
    )?;
    systemctl(&["daemon-reload"]);
    Ok(())
}

/// Grava o Quadlet e baixa a imagem, com o progresso do `podman pull`.
#[tauri::command]
pub fn containers_install(
    app: AppHandle,
    id: String,
    accept_license: Option<bool>,
) -> Result<u32, String> {
    let service = service(&id).ok_or_else(|| gettext("Serviço desconhecido."))?;
    // O aceite é do usuário, no diálogo: sem ele, nada é gravado.
    if service.license.is_some() && accept_license != Some(true) {
        return Err(gettext("É preciso aceitar a licença para instalar."));
    }
    prepare(service)?;
    let tx = super::session_tx::next();
    std::thread::spawn(move || {
        let mut command = Command::new("podman");
        command.args(["pull", service.image]);
        crate::webkit_env::clean_command(&mut command);
        follow_then(
            &app,
            tx,
            command,
            "podman",
            gettext(
                "O download da imagem falhou. Tente de novo: o Podman reaproveita o que já baixou.",
            ),
            |_| {},
        );
    });
    Ok(tx)
}

/// Iniciar ou parar. Sem bloquear: a primeira partida cria o banco e leva
/// minutos; a interface acompanha pelo estado e pela saúde do contêiner.
#[tauri::command]
pub fn containers_action(id: String, action: String) -> Result<(), String> {
    let service = service(&id).ok_or_else(|| gettext("Serviço desconhecido."))?;
    let verb = match action.as_str() {
        "start" => "start",
        "stop" => "stop",
        _ => return Err(gettext("Ação desconhecida.")),
    };
    let name = format!("{}.service", unit(service));
    if systemctl(&["--no-block", verb, &name]) {
        Ok(())
    } else {
        Err(gettext("O systemd do usuário recusou a ação."))
    }
}

/// Remove o serviço. `keep_data`: o volume fica, e reinstalar o reaproveita
/// (com a mesma senha).
#[tauri::command]
pub fn containers_remove(id: String, keep_data: bool) -> Result<(), String> {
    let service = service(&id).ok_or_else(|| gettext("Serviço desconhecido."))?;
    let config = config_dir().ok_or_else(|| gettext("Pasta pessoal não encontrada."))?;
    let name = unit(service);
    systemctl(&["stop", &format!("{name}.service")]);
    let _ = std::fs::remove_file(quadlet_path(&config, service));
    systemctl(&["daemon-reload"]);
    let _ = run("podman", &["rm", "--ignore", &name]);
    let _ = run("podman", &["rmi", "--ignore", service.image]);
    if !keep_data {
        let _ = run("podman", &["volume", "rm", "--force", &volume(service)]);
        let _ = std::fs::remove_file(env_path(&config, service));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadlet_publishes_only_on_localhost_and_never_autostarts() {
        let oracle = service("oracle-free").unwrap();
        let text = quadlet(
            oracle,
            Path::new("/home/u/.config/lyra-containers/oracle-free.env"),
        );
        assert!(text.contains("ContainerName=lyra-oracle-free\n"));
        assert!(text.contains("Image=container-registry.oracle.com/database/free:23.26.3.0\n"));
        assert!(text.contains("Volume=lyra-oracle-free-data:/opt/oracle/oradata\n"));
        assert!(text.contains("PublishPort=127.0.0.1:1521:1521\n"));
        assert!(text.contains("EnvironmentFile=/home/u/.config/lyra-containers/oracle-free.env\n"));
        assert!(!text.contains("[Install]"));
    }

    #[test]
    fn sql_server_quadlet_accepts_license_and_checks_health() {
        let mssql = service("mssql").unwrap();
        let text = quadlet(
            mssql,
            Path::new("/home/u/.config/lyra-containers/mssql.env"),
        );
        assert!(text.contains("PublishPort=127.0.0.1:1433:1433\n"));
        assert!(text.contains("Volume=lyra-mssql-data:/var/opt/mssql\n"));
        assert!(text.contains("Environment=MSSQL_PID=Developer\n"));
        assert!(text.contains("Environment=ACCEPT_EULA=Y\n"));
        assert!(text.contains("HealthCmd=/opt/mssql-tools18/bin/sqlcmd "));
        // O Oracle não tem licença a aceitar nem verificação própria.
        let oracle = quadlet(service("oracle-free").unwrap(), Path::new("/x.env"));
        assert!(!oracle.contains("ACCEPT_EULA") && !oracle.contains("HealthCmd"));
    }

    #[test]
    fn license_must_be_accepted() {
        assert!(service("mssql").unwrap().license.is_some());
        assert!(service("oracle-free").unwrap().license.is_none());
    }

    #[test]
    fn unknown_services_are_refused() {
        assert!(service("docker.io/qualquer").is_none());
        assert!(containers_password("../../etc".into()).is_err());
        assert!(containers_action("oracle-free".into(), "rm -rf".into()).is_err());
    }

    #[test]
    fn images_are_pinned_and_ids_are_plain() {
        for service in SERVICES {
            assert!(!service.image.ends_with(":latest"), "{}", service.id);
            assert!(service.image.contains(':'), "{}", service.id);
            assert!(
                service
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{}",
                service.id
            );
        }
    }

    #[test]
    fn password_has_upper_lower_and_digit() {
        let password = random_password().unwrap();
        assert!(password.len() >= 12);
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
