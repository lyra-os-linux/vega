use async_trait::async_trait;
use futures_util::{FutureExt, StreamExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub id: u32,
    pub timestamp: i64,
    pub trigger: String,
    pub description: String,
}

impl From<(u32, i64, String, String)> for Snapshot {
    fn from(row: (u32, i64, String, String)) -> Self {
        Self {
            id: row.0,
            timestamp: row.1,
            trigger: row.2,
            description: row.3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotsClientError(String);

impl std::fmt::Display for SnapshotsClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            gettextrs::gettext("interface de snapshots indisponível: {detail}")
                .replace("{detail}", &self.0)
        )
    }
}

impl std::error::Error for SnapshotsClientError {}

impl SnapshotsClientError {
    fn from_error(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

#[async_trait]
pub trait SnapshotsClient: Send + Sync {
    async fn available(&self) -> Result<bool, SnapshotsClientError>;
    async fn list(&self) -> Result<Vec<Snapshot>, SnapshotsClientError>;
    async fn create(&self, description: &str) -> Result<u32, SnapshotsClientError>;
    async fn diff_packages(&self, id: u32) -> Result<Vec<String>, SnapshotsClientError>;
    async fn rollback(&self, id: u32) -> Result<(), SnapshotsClientError>;
    async fn delete(&self, id: u32) -> Result<(), SnapshotsClientError>;
    async fn clear(&self) -> Result<u32, SnapshotsClientError>;
    async fn set_retention(&self, keep: u32) -> Result<(), SnapshotsClientError>;
}

#[zbus::proxy(
    interface = "org.lyraos.Vega1.Snapshots",
    default_service = "org.lyraos.Vega1",
    default_path = "/org/lyraos/Vega1"
)]
trait Snapshots {
    async fn available(&self) -> zbus::Result<bool>;
    async fn list_snapshots(&self) -> zbus::Result<Vec<(u32, i64, String, String)>>;
    async fn create_snapshot(&self, description: &str) -> zbus::Result<u32>;
    async fn diff_packages(&self, snapshot_id: u32) -> zbus::Result<Vec<String>>;
    async fn diff_packages_localized(
        &self,
        snapshot_id: u32,
        locale: &str,
    ) -> zbus::Result<Vec<String>>;
    async fn rollback(&self, snapshot_id: u32) -> zbus::Result<()>;
    async fn delete_snapshot(&self, snapshot_id: u32) -> zbus::Result<()>;
    async fn clear_snapshots(&self) -> zbus::Result<u32>;
    async fn set_retention_policy(&self, keep_count: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn clear_progress(&self, deleted_count: u32, total_count: u32) -> zbus::Result<()>;
}

pub struct ZbusSnapshotsClient {
    connection: zbus::Connection,
}

impl ZbusSnapshotsClient {
    pub fn from_connection(connection: zbus::Connection) -> Self {
        Self { connection }
    }

    async fn proxy(&self) -> Result<SnapshotsProxy<'_>, SnapshotsClientError> {
        SnapshotsProxy::new(&self.connection)
            .await
            .map_err(SnapshotsClientError::from_error)
    }

    pub async fn clear_with_progress<F>(
        &self,
        mut on_progress: F,
    ) -> Result<u32, SnapshotsClientError>
    where
        F: FnMut(u32, u32),
    {
        let proxy = self.proxy().await?;
        let mut progress = proxy
            .receive_clear_progress()
            .await
            .map_err(SnapshotsClientError::from_error)?;
        let clear = proxy.clear_snapshots().fuse();
        futures_util::pin_mut!(clear);
        loop {
            futures_util::select! {
                result = clear => return result.map_err(SnapshotsClientError::from_error),
                signal = progress.next().fuse() => {
                    let signal = signal.ok_or_else(|| SnapshotsClientError::from_error("fluxo de progresso encerrado"))?;
                    let args = signal.args().map_err(SnapshotsClientError::from_error)?;
                    on_progress(args.deleted_count, args.total_count);
                }
            }
        }
    }
}

macro_rules! call {
    ($self:ident, $method:ident ( $($arg:expr),* $(,)? )) => {
        $self.proxy().await?.$method($($arg),*).await.map_err(SnapshotsClientError::from_error)
    };
}

#[async_trait]
impl SnapshotsClient for ZbusSnapshotsClient {
    async fn available(&self) -> Result<bool, SnapshotsClientError> {
        call!(self, available())
    }
    async fn list(&self) -> Result<Vec<Snapshot>, SnapshotsClientError> {
        call!(self, list_snapshots()).map(|rows| rows.into_iter().map(Into::into).collect())
    }
    async fn create(&self, description: &str) -> Result<u32, SnapshotsClientError> {
        call!(self, create_snapshot(description))
    }
    async fn diff_packages(&self, id: u32) -> Result<Vec<String>, SnapshotsClientError> {
        call!(self, diff_packages_localized(id, crate::current_locale()))
    }
    async fn rollback(&self, id: u32) -> Result<(), SnapshotsClientError> {
        call!(self, rollback(id))
    }
    async fn delete(&self, id: u32) -> Result<(), SnapshotsClientError> {
        call!(self, delete_snapshot(id))
    }
    async fn clear(&self) -> Result<u32, SnapshotsClientError> {
        call!(self, clear_snapshots())
    }
    async fn set_retention(&self, keep: u32) -> Result<(), SnapshotsClientError> {
        call!(self, set_retention_policy(keep))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn snapshots_xml_contains_every_typed_method() {
        let xml = include_str!("../../dbus/org.lyraos.Vega1.Snapshots.xml");
        let start = xml.find("<node").unwrap();
        let document = roxmltree::Document::parse(&xml[start..]).unwrap();
        let mut methods = document
            .descendants()
            .filter(|node| node.has_tag_name("method"))
            .map(|node| node.attribute("name").unwrap())
            .collect::<Vec<_>>();
        methods.sort_unstable();
        assert_eq!(
            methods,
            [
                "Available",
                "ClearSnapshots",
                "CreateSnapshot",
                "DeleteSnapshot",
                "DiffPackages",
                "DiffPackagesLocalized",
                "ListSnapshots",
                "Rollback",
                "SetRetentionPolicy",
            ]
        );
    }
}
