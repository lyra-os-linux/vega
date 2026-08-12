use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareInventory {
    pub cpu: String,
    pub gpu: String,
    pub ram: String,
}

impl From<(String, String, String)> for HardwareInventory {
    fn from(row: (String, String, String)) -> Self {
        Self {
            cpu: row.0,
            gpu: row.1,
            ram: row.2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareClientError {
    Unavailable(String),
}

impl std::fmt::Display for HardwareClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(detail) => write!(
                f,
                "{}",
                gettextrs::gettext("interface de hardware indisponível: {detail}")
                    .replace("{detail}", detail)
            ),
        }
    }
}

impl std::error::Error for HardwareClientError {}

#[async_trait]
pub trait HardwareClient: Send + Sync {
    async fn inventory(&self) -> Result<HardwareInventory, HardwareClientError>;
    async fn firmware_status(&self) -> Result<String, HardwareClientError>;
    async fn switch_nvidia_driver(&self, driver: &str) -> Result<(), HardwareClientError>;
}

#[zbus::proxy(
    interface = "org.lyraos.Vega1.Hardware",
    default_service = "org.lyraos.Vega1",
    default_path = "/org/lyraos/Vega1"
)]
trait Hardware {
    async fn inventory(&self) -> zbus::Result<(String, String, String)>;
    async fn firmware_status(&self) -> zbus::Result<String>;
    async fn inventory_localized(&self, locale: &str) -> zbus::Result<(String, String, String)>;
    async fn firmware_status_localized(&self, locale: &str) -> zbus::Result<String>;
    async fn switch_nvidia_driver(&self, driver: &str) -> zbus::Result<()>;
}

pub struct ZbusHardwareClient {
    connection: zbus::Connection,
}

impl ZbusHardwareClient {
    pub async fn connect() -> Result<Self, HardwareClientError> {
        let connection = zbus::Connection::system()
            .await
            .map_err(HardwareClientError::unavailable)?;
        Ok(Self { connection })
    }

    pub fn from_connection(connection: zbus::Connection) -> Self {
        Self { connection }
    }

    async fn proxy(&self) -> Result<HardwareProxy<'_>, HardwareClientError> {
        HardwareProxy::new(&self.connection)
            .await
            .map_err(HardwareClientError::unavailable)
    }
}

impl HardwareClientError {
    fn unavailable(error: impl std::fmt::Display) -> Self {
        Self::Unavailable(error.to_string())
    }
}

#[async_trait]
impl HardwareClient for ZbusHardwareClient {
    async fn inventory(&self) -> Result<HardwareInventory, HardwareClientError> {
        self.proxy()
            .await?
            .inventory_localized(crate::current_locale())
            .await
            .map(Into::into)
            .map_err(HardwareClientError::unavailable)
    }

    async fn firmware_status(&self) -> Result<String, HardwareClientError> {
        self.proxy()
            .await?
            .firmware_status_localized(crate::current_locale())
            .await
            .map_err(HardwareClientError::unavailable)
    }

    async fn switch_nvidia_driver(&self, driver: &str) -> Result<(), HardwareClientError> {
        self.proxy()
            .await?
            .switch_nvidia_driver(driver)
            .await
            .map_err(HardwareClientError::unavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::HardwareClient;

    const HARDWARE_XML: &str = include_str!("../../dbus/org.lyraos.Vega1.Hardware.xml");

    #[test]
    fn hardware_xml_matches_the_typed_proxy() {
        let node_start = HARDWARE_XML.find("<node").expect("nó de introspecção");
        let document =
            roxmltree::Document::parse(&HARDWARE_XML[node_start..]).expect("Hardware XML válido");
        let interface = document
            .descendants()
            .find(|node| {
                node.has_tag_name("interface")
                    && node.attribute("name") == Some("org.lyraos.Vega1.Hardware")
            })
            .expect("interface Hardware presente");
        let methods: BTreeMap<_, _> = interface
            .children()
            .filter(|node| node.has_tag_name("method"))
            .map(|method| {
                let args = method
                    .children()
                    .filter(|node| node.has_tag_name("arg"))
                    .map(|arg| {
                        (
                            arg.attribute("direction").unwrap_or("in"),
                            arg.attribute("type").unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                (method.attribute("name").unwrap(), args)
            })
            .collect();
        let expected = BTreeMap::from([
            ("FirmwareStatus", vec![("out", "s")]),
            ("FirmwareStatusLocalized", vec![("in", "s"), ("out", "s")]),
            ("Inventory", vec![("out", "(sss)")]),
            ("InventoryLocalized", vec![("in", "s"), ("out", "(sss)")]),
            ("SwitchNvidiaDriver", vec![("in", "s")]),
        ]);
        assert_eq!(methods, expected);
    }

    #[test]
    fn inventory_tuple_maps_to_named_fields() {
        let inventory =
            super::HardwareInventory::from(("CPU".into(), "GPU".into(), "16 GiB".into()));
        assert_eq!(inventory.cpu, "CPU");
        assert_eq!(inventory.gpu, "GPU");
        assert_eq!(inventory.ram, "16 GiB");
    }

    #[test]
    #[ignore = "requer vegad instalado e acesso ao system bus"]
    fn real_daemon_exposes_hardware_inventory() {
        futures_lite::future::block_on(async {
            let client = super::ZbusHardwareClient::connect().await.unwrap();
            let inventory = client.inventory().await.unwrap();
            assert!(!inventory.cpu.is_empty());
            assert!(!inventory.ram.is_empty());
        });
    }
}
