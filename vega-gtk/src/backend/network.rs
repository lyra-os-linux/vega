use lyra_vega_dbus::{
    FirewallClient, FirewallPort, FirewallService, FirewallStatus, NetworkClient, NetworkInterface,
    ProxyConfig, WifiNetwork,
};

use super::{CmdResult, State, dto, err, list};

dto!(Interface from NetworkInterface {
    name: String, kind: String, state: String, ipv4: String, ipv6: String, gateway: String,
    dns: String, mac: String, speed: String, ssid: String, signal: u32, device: String, autoconf: bool,
});
dto!(Wifi from WifiNetwork { ssid: String, security: String, signal: u32, active: bool, device: String });
dto!(Proxy from ProxyConfig { http: String, https: String, socks: String, no_proxy: String });
dto!(Firewall from FirewallStatus { enabled: bool, active_zone: String });
dto!(Service from FirewallService { name: String, label: String, enabled: bool });
dto!(Port from FirewallPort { port: String, protocol: String });

#[tauri::command]
pub async fn network_interfaces(state: State<'_>) -> CmdResult<Vec<Interface>> {
    state
        .dbus()
        .await?
        .network()
        .interfaces()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn network_wifi(state: State<'_>) -> CmdResult<Vec<Wifi>> {
    state
        .dbus()
        .await?
        .network()
        .wifi()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn network_proxy(state: State<'_>) -> CmdResult<Proxy> {
    state
        .dbus()
        .await?
        .network()
        .proxy()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn network_set_proxy(state: State<'_>, proxy: Proxy) -> CmdResult<()> {
    let config = ProxyConfig {
        http: proxy.http,
        https: proxy.https,
        socks: proxy.socks,
        no_proxy: proxy.no_proxy,
    };
    state
        .dbus()
        .await?
        .network()
        .set_proxy(&config)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn network_connect_wifi(
    state: State<'_>,
    ssid: String,
    password: String,
) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .network()
        .connect_wifi(&ssid, &password)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn network_disconnect(state: State<'_>, device: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .network()
        .disconnect(&device)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn network_set_static_ipv4(
    state: State<'_>,
    connection: String,
    address: String,
    gateway: String,
    dns: String,
) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .network()
        .set_static_ipv4(&connection, &address, &gateway, &dns)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn network_import_vpn(state: State<'_>, path: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .network()
        .import_vpn(&path)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_status(state: State<'_>) -> CmdResult<Firewall> {
    state
        .dbus()
        .await?
        .firewall()
        .status()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_services(state: State<'_>) -> CmdResult<Vec<Service>> {
    state
        .dbus()
        .await?
        .firewall()
        .services()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_ports(state: State<'_>) -> CmdResult<Vec<Port>> {
    state
        .dbus()
        .await?
        .firewall()
        .ports()
        .await
        .map(list)
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_set_service(state: State<'_>, name: String, enabled: bool) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .firewall()
        .set_service(&name, enabled)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_add_port(state: State<'_>, port: String, protocol: String) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .firewall()
        .add_port(&port, &protocol)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn firewall_remove_port(
    state: State<'_>,
    port: String,
    protocol: String,
) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .firewall()
        .remove_port(&port, &protocol)
        .await
        .map_err(err)
}
