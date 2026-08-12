use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedService {
    pub name: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub active: bool,
    pub available: bool,
}

impl From<(String, String, String, bool, bool, bool)> for ManagedService {
    fn from(row: (String, String, String, bool, bool, bool)) -> Self {
        Self {
            name: row.0,
            label: row.1,
            description: row.2,
            enabled: row.3,
            active: row.4,
            available: row.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicesClientError(String);

impl std::fmt::Display for ServicesClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            gettextrs::gettext("interface de serviços indisponível: {detail}")
                .replace("{detail}", &self.0)
        )
    }
}

impl std::error::Error for ServicesClientError {}

impl ServicesClientError {
    fn from_error(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

type ServiceRow = (String, String, String, bool, bool, bool);

#[async_trait]
pub trait ServicesClient: Send + Sync {
    async fn list(&self) -> Result<Vec<ManagedService>, ServicesClientError>;
    async fn list_all(&self) -> Result<Vec<ManagedService>, ServicesClientError>;
    async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), ServicesClientError>;
    async fn set_running(&self, name: &str, running: bool) -> Result<(), ServicesClientError>;
    async fn restart(&self, name: &str) -> Result<(), ServicesClientError>;
}

#[zbus::proxy(
    interface = "org.lyraos.Vega1.Services",
    default_service = "org.lyraos.Vega1",
    default_path = "/org/lyraos/Vega1"
)]
trait Services {
    async fn list_services(&self) -> zbus::Result<Vec<ServiceRow>>;
    async fn list_all_services(&self) -> zbus::Result<Vec<ServiceRow>>;
    async fn list_services_localized(&self, locale: &str) -> zbus::Result<Vec<ServiceRow>>;
    async fn list_all_services_localized(&self, locale: &str) -> zbus::Result<Vec<ServiceRow>>;
    async fn set_service_enabled(&self, name: &str, enabled: bool) -> zbus::Result<()>;
    async fn set_service_running(&self, name: &str, running: bool) -> zbus::Result<()>;
    async fn restart_service(&self, name: &str) -> zbus::Result<()>;
}

pub struct ZbusServicesClient {
    connection: zbus::Connection,
}

impl ZbusServicesClient {
    pub fn from_connection(connection: zbus::Connection) -> Self {
        Self { connection }
    }
    async fn proxy(&self) -> Result<ServicesProxy<'_>, ServicesClientError> {
        ServicesProxy::new(&self.connection)
            .await
            .map_err(ServicesClientError::from_error)
    }
}

macro_rules! call {
    ($self:ident, $method:ident ( $($arg:expr),* $(,)? )) => {
        $self.proxy().await?.$method($($arg),*).await.map_err(ServicesClientError::from_error)
    };
}

fn convert(rows: Vec<ServiceRow>) -> Vec<ManagedService> {
    rows.into_iter().map(Into::into).collect()
}

#[async_trait]
impl ServicesClient for ZbusServicesClient {
    async fn list(&self) -> Result<Vec<ManagedService>, ServicesClientError> {
        call!(self, list_services_localized(crate::current_locale())).map(convert)
    }
    async fn list_all(&self) -> Result<Vec<ManagedService>, ServicesClientError> {
        call!(self, list_all_services_localized(crate::current_locale())).map(convert)
    }
    async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), ServicesClientError> {
        call!(self, set_service_enabled(name, enabled))
    }
    async fn set_running(&self, name: &str, running: bool) -> Result<(), ServicesClientError> {
        call!(self, set_service_running(name, running))
    }
    async fn restart(&self, name: &str) -> Result<(), ServicesClientError> {
        call!(self, restart_service(name))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn services_xml_contains_every_typed_method() {
        let xml = include_str!("../../dbus/org.lyraos.Vega1.Services.xml");
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
                "ListAllServices",
                "ListAllServicesLocalized",
                "ListServices",
                "ListServicesLocalized",
                "RestartService",
                "SetServiceEnabled",
                "SetServiceRunning",
            ]
        );
    }
}
