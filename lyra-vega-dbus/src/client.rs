use super::{
    ZbusBackupClient, ZbusBluetoothClient, ZbusDateTimeClient, ZbusFirewallClient,
    ZbusHardwareClient, ZbusKernelClient, ZbusLogsClient, ZbusMetadataClient, ZbusMonitorClient,
    ZbusNetworkClient, ZbusServicesClient, ZbusSnapshotsClient, ZbusSoftwareClient,
    ZbusStorageClient, ZbusSystemClient, ZbusUsersClient,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbusConnectionError(String);

impl std::fmt::Display for DbusConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            gettextrs::gettext("não foi possível conectar ao system bus: {detail}")
                .replace("{detail}", &self.0)
        )
    }
}

impl std::error::Error for DbusConnectionError {}

#[derive(Clone)]
pub struct VegaDbus {
    connection: zbus::Connection,
}

impl VegaDbus {
    pub async fn connect() -> Result<Self, DbusConnectionError> {
        let connection = zbus::Connection::system()
            .await
            .map_err(|error| DbusConnectionError(error.to_string()))?;
        Ok(Self { connection })
    }

    pub fn system(&self) -> ZbusSystemClient {
        ZbusSystemClient::from_connection(self.connection.clone())
    }

    pub fn metadata(&self) -> ZbusMetadataClient {
        ZbusMetadataClient::from_connection(self.connection.clone())
    }

    pub fn software(&self) -> ZbusSoftwareClient {
        ZbusSoftwareClient::from_connection(self.connection.clone())
    }

    pub fn backup(&self) -> ZbusBackupClient {
        ZbusBackupClient::from_connection(self.connection.clone())
    }

    pub fn bluetooth(&self) -> ZbusBluetoothClient {
        ZbusBluetoothClient::from_connection(self.connection.clone())
    }

    pub fn hardware(&self) -> ZbusHardwareClient {
        ZbusHardwareClient::from_connection(self.connection.clone())
    }

    pub fn kernel(&self) -> ZbusKernelClient {
        ZbusKernelClient::from_connection(self.connection.clone())
    }

    pub fn datetime(&self) -> ZbusDateTimeClient {
        ZbusDateTimeClient::from_connection(self.connection.clone())
    }

    pub fn storage(&self) -> ZbusStorageClient {
        ZbusStorageClient::from_connection(self.connection.clone())
    }

    pub fn network(&self) -> ZbusNetworkClient {
        ZbusNetworkClient::from_connection(self.connection.clone())
    }

    pub fn firewall(&self) -> ZbusFirewallClient {
        ZbusFirewallClient::from_connection(self.connection.clone())
    }

    pub fn snapshots(&self) -> ZbusSnapshotsClient {
        ZbusSnapshotsClient::from_connection(self.connection.clone())
    }

    pub fn services(&self) -> ZbusServicesClient {
        ZbusServicesClient::from_connection(self.connection.clone())
    }

    pub fn users(&self) -> ZbusUsersClient {
        ZbusUsersClient::from_connection(self.connection.clone())
    }

    pub fn logs(&self) -> ZbusLogsClient {
        ZbusLogsClient::from_connection(self.connection.clone())
    }

    pub fn monitor(&self) -> ZbusMonitorClient {
        ZbusMonitorClient::from_connection(self.connection.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MetadataClient, ServicesClient, SnapshotsClient};

    #[test]
    #[ignore = "requer vegad instalado e acesso ao system bus"]
    fn dashboard_read_clients_work_against_the_real_daemon() {
        futures_lite::future::block_on(async {
            let dbus = VegaDbus::connect().await.unwrap();
            let metadata = dbus.metadata().metadata().await.unwrap();
            assert!(matches!(metadata.profile.as_str(), "desktop" | "server"));
            assert!(
                metadata
                    .capabilities
                    .iter()
                    .any(|item| item == "packages-native")
            );
            dbus.snapshots().available().await.unwrap();
            dbus.services().list().await.unwrap();
        });
    }
}
