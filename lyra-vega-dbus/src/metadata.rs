use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonMetadata {
    pub profile: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataClientError(pub String);

impl std::fmt::Display for MetadataClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "metadados do vegad indisponíveis: {}", self.0)
    }
}

impl std::error::Error for MetadataClientError {}

#[async_trait]
pub trait MetadataClient: Send + Sync {
    async fn metadata(&self) -> Result<DaemonMetadata, MetadataClientError>;
}

#[zbus::proxy(
    interface = "org.lyraos.Vega1.Metadata",
    default_service = "org.lyraos.Vega1",
    default_path = "/org/lyraos/Vega1"
)]
trait Metadata {
    async fn profile(&self) -> zbus::Result<String>;
    async fn version(&self) -> zbus::Result<String>;
    async fn capabilities(&self) -> zbus::Result<Vec<String>>;
}

pub struct ZbusMetadataClient {
    connection: zbus::Connection,
}

impl ZbusMetadataClient {
    pub fn from_connection(connection: zbus::Connection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl MetadataClient for ZbusMetadataClient {
    async fn metadata(&self) -> Result<DaemonMetadata, MetadataClientError> {
        let proxy = MetadataProxy::new(&self.connection)
            .await
            .map_err(|e| MetadataClientError(e.to_string()))?;
        let (profile, version, capabilities) =
            futures_util::try_join!(proxy.profile(), proxy.version(), proxy.capabilities())
                .map_err(|e| MetadataClientError(e.to_string()))?;
        Ok(DaemonMetadata {
            profile,
            version,
            capabilities,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_xml_has_stable_contract() {
        let xml = include_str!("../../dbus/org.lyraos.Vega1.Metadata.xml");
        assert!(xml.contains("org.lyraos.Vega1.Metadata"));
        assert!(xml.contains("method name=\"Profile\""));
        assert!(xml.contains("method name=\"Capabilities\""));
    }
}
