//! Real GTK callbacks and D-Bus payloads, without a real daemon or user settings.
use super::*;
use glib::variant::ToVariant;
use std::{io::BufRead, process::Command, time::Duration};

type Calls = Rc<RefCell<Vec<(String, glib::Variant)>>>;

struct PrivateBus(std::process::Child);
impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn isolated_child(test: &str) {
    let home = std::env::temp_dir().join(format!("vega-dialogs-{}", std::process::id()));
    std::fs::create_dir(&home).unwrap();
    let config = home.join("bus.conf");
    std::fs::write(&config, r#"<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen><auth>EXTERNAL</auth><policy context="default"><allow send_destination="*"/><allow receive_sender="*"/><allow own="*"/></policy></busconfig>"#).unwrap();
    let mut bus = PrivateBus(
        Command::new("dbus-daemon")
            .arg("--config-file")
            .arg(&config)
            .args(["--nofork", "--print-address=1"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut address = String::new();
    std::io::BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();

    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            test,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("VEGA_DIALOG_TEST_BUS", address.trim())
        .env("DBUS_SYSTEM_BUS_ADDRESS", address.trim())
        .env("DBUS_SESSION_BUS_ADDRESS", address.trim())
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("GSETTINGS_BACKEND", "memory")
        .env("GIO_USE_VFS", "local")
        .env("GTK_A11Y", "none")
        .status();
    std::fs::remove_dir_all(home).unwrap();
    assert!(result.unwrap().success());
}

fn fixture(address: &str, calls: &Calls) -> gio::DBusConnection {
    fixture_with_network(address, calls, None)
}

#[derive(Clone, Default)]
struct NetworkFixture {
    pending: Rc<RefCell<Vec<gio::DBusMethodInvocation>>>,
    list_reply: Rc<RefCell<Option<glib::Variant>>>,
}

fn fixture_with_network(
    address: &str,
    calls: &Calls,
    network: Option<NetworkFixture>,
) -> gio::DBusConnection {
    fixture_with_services(address, calls, network, None)
}

#[derive(Clone)]
struct NvidiaFixture {
    state: Rc<RefCell<String>>,
    capable: Rc<Cell<bool>>,
    result: Rc<Cell<u8>>,
}

fn fixture_with_services(
    address: &str,
    calls: &Calls,
    network: Option<NetworkFixture>,
    nvidia: Option<NvidiaFixture>,
) -> gio::DBusConnection {
    fixture_with_updates(address, calls, network, nvidia, None)
}

type PendingUpdates = Rc<RefCell<Vec<gio::DBusMethodInvocation>>>;

fn fixture_with_updates(
    address: &str,
    calls: &Calls,
    network: Option<NetworkFixture>,
    nvidia: Option<NvidiaFixture>,
    updates: Option<PendingUpdates>,
) -> gio::DBusConnection {
    assert_eq!(std::env::var("DBUS_SYSTEM_BUS_ADDRESS").unwrap(), address);
    let connection = gio::DBusConnection::for_address_sync(
        address,
        gio::DBusConnectionFlags::AUTHENTICATION_CLIENT
            | gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION,
        None,
        gio::Cancellable::NONE,
    )
    .unwrap();
    connection
        .call_sync(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "RequestName",
            Some(&("org.lyraos.Vega1", 0u32).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            2000,
            gio::Cancellable::NONE,
        )
        .unwrap();
    for (interface, methods) in [
        (
            "Backup",
            vec![
                ("ListConfigs", vec![], "a(sassss)"),
                ("CreateConfig", vec!["(sassss)"], "s"),
            ],
        ),
        (
            "Snapshots",
            vec![
                ("Available", vec![], "b"),
                ("ListSnapshots", vec![], "a(uxss)"),
                ("CreateSnapshot", vec!["s"], "u"),
                ("DiffPackagesLocalized", vec!["u", "s"], "as"),
                ("Rollback", vec!["u"], ""),
            ],
        ),
        (
            "Network",
            vec![
                ("ListInterfaces", vec![], "a(ssssssssssusb)"),
                ("SetStaticIpv4", vec!["s", "s", "s", "s"], ""),
                ("ConnectWifi", vec!["s", "s"], ""),
            ],
        ),
        (
            "Software",
            vec![
                ("AddRepo", vec!["s", "s"], "u"),
                ("Install", vec!["s", "s"], "u"),
                ("Remove", vec!["s", "s"], "u"),
                ("ClearCache", vec![], "u"),
                ("TrustRepoKey", vec!["s", "s"], "u"),
                ("NvidiaStatus", vec![], "(bbbssssu)"),
                ("InstallNvidia", vec!["b"], "u"),
            ],
        ),
        (
            "Metadata",
            vec![
                ("Profile", vec![], "s"),
                ("Version", vec![], "s"),
                ("Capabilities", vec![], "as"),
            ],
        ),
    ] {
        let mut methods = methods;
        if interface == "Software" && updates.is_some() {
            methods.push(("ListUpdates", vec![], "a(ssssbss)"));
        }
        let mut xml = format!("<node><interface name='org.lyraos.Vega1.{interface}'>");
        for (method, inputs, output) in methods {
            xml.push_str(&format!("<method name='{method}'>"));
            for input in inputs {
                xml.push_str(&format!("<arg type='{input}' direction='in'/>"));
            }
            if !output.is_empty() {
                xml.push_str(&format!("<arg type='{output}' direction='out'/>"));
            }
            xml.push_str("</method>");
        }
        xml.push_str("</interface></node>");
        let info = gio::DBusNodeInfo::for_xml(&xml).unwrap();
        let calls = calls.clone();
        let network = network.clone();
        let nvidia = nvidia.clone();
        let updates = updates.clone();
        connection
            .register_object("/org/lyraos/Vega1", &info.interfaces()[0])
            .method_call(move |conn, _, _, _, method, parameters, invocation| {
                let value = match method {
                    "ListUpdates" if updates.is_some() => {
                        updates.as_ref().unwrap().borrow_mut().push(invocation);
                        return;
                    }
                    "Profile" => Some(("desktop",).to_variant()),
                    "Version" => Some(("5.1.28",).to_variant()),
                    "Capabilities" => Some(
                        (if nvidia.as_ref().is_some_and(|n| n.capable.get()) {
                            vec!["nvidia-official-v1"]
                        } else {
                            vec![]
                        },)
                            .to_variant(),
                    ),
                    "NvidiaStatus" if nvidia.is_some() => {
                        let state = nvidia.as_ref().unwrap().state.borrow().clone();
                        Some(
                            ((
                                true,
                                state == "active",
                                false,
                                "NVIDIA GTX 1650",
                                "enabled",
                                state,
                                "NVIDIA 610.57.04",
                                0u32,
                            ),)
                                .to_variant(),
                        )
                    }
                    "InstallNvidia"
                        if nvidia.is_some() && nvidia.as_ref().unwrap().result.get() != 0 =>
                    {
                        calls.borrow_mut().push((method.into(), parameters));
                        invocation.return_value(Some(&(73u32,).to_variant()));
                        let n = nvidia.as_ref().unwrap().clone();
                        glib::timeout_add_local_once(Duration::from_millis(50), move || {
                            if n.result.get() == 3 {
                                conn.call_sync(
                                    Some("org.freedesktop.DBus"),
                                    "/org/freedesktop/DBus",
                                    "org.freedesktop.DBus",
                                    "ReleaseName",
                                    Some(&("org.lyraos.Vega1",).to_variant()),
                                    None,
                                    gio::DBusCallFlags::NONE,
                                    2000,
                                    gio::Cancellable::NONE,
                                )
                                .unwrap();
                            } else {
                                let success = n.result.get() == 1;
                                *n.state.borrow_mut() =
                                    if success { "active" } else { "inconsistent" }.into();
                                conn.emit_signal(
                                    None,
                                    "/org/lyraos/Vega1",
                                    "org.lyraos.Vega1.Software",
                                    "TransactionProgress",
                                    Some(&(73u32, 50u32, "fixture").to_variant()),
                                )
                                .unwrap();
                                conn.emit_signal(
                                    None,
                                    "/org/lyraos/Vega1",
                                    "org.lyraos.Vega1.Software",
                                    "TransactionFinished",
                                    Some(&(73u32, success, "fixture result").to_variant()),
                                )
                                .unwrap();
                            }
                        });
                        return;
                    }
                    "ListConfigs" => Some(glib::Variant::parse(None, "(@a(sassss) [],)").unwrap()),
                    "Available" => Some((true,).to_variant()),
                    "ListSnapshots" => Some(glib::Variant::parse(None, "(@a(uxss) [],)").unwrap()),
                    "DiffPackagesLocalized" => {
                        Some((vec!["fixture-package: 2 -> 1"],).to_variant())
                    }
                    "ListInterfaces" => network
                        .as_ref()
                        .and_then(|network| network.list_reply.borrow().clone()),
                    "SetStaticIpv4" if network.is_some() => {
                        calls.borrow_mut().push((method.into(), parameters));
                        network
                            .as_ref()
                            .unwrap()
                            .pending
                            .borrow_mut()
                            .push(invocation);
                        return;
                    }
                    _ => {
                        calls.borrow_mut().push((method.into(), parameters));
                        None
                    }
                };
                if let Some(value) = value {
                    invocation.return_value(Some(&value));
                } else {
                    // Record the exact request but never perform a mutation.
                    invocation.return_dbus_error("org.lyraos.Test.Rejected", "fixture rejected");
                }
            })
            .build()
            .unwrap();
    }
    connection
}

fn descendants(widget: &impl IsA<gtk::Widget>) -> Vec<gtk::Widget> {
    let mut result = vec![widget.as_ref().clone()];
    let mut child = widget.as_ref().first_child();
    while let Some(current) = child {
        result.extend(descendants(&current));
        child = current.next_sibling();
    }
    result
}

async fn until(description: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out: {description}");
        glib::timeout_future(Duration::from_millis(10)).await;
    }
}

fn active_dialog() -> Option<adw::AlertDialog> {
    gtk::Window::list_toplevels()
        .iter()
        .flat_map(descendants)
        .filter_map(|widget| widget.downcast::<adw::AlertDialog>().ok())
        .find(|dialog| dialog.is_mapped())
}

async fn dialog(calls: &Calls, before: usize) -> adw::AlertDialog {
    until("form must be displayed before processing input", || {
        active_dialog().is_some()
    })
    .await;
    assert_eq!(calls.borrow().len(), before, "request before user response");
    active_dialog().unwrap()
}

async fn respond(dialog: &adw::AlertDialog, response: &str) {
    let label = dialog.response_label(response);
    let button = descendants(dialog)
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Button>().ok())
        .find(|button| button.label().as_deref() == Some(label.as_str()))
        .unwrap();
    button.emit_clicked();
    until("dialog closed", || !dialog.is_mapped()).await;
}

fn entries(dialog: &adw::AlertDialog) -> Vec<gtk::Entry> {
    descendants(&dialog.extra_child().unwrap())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Entry>().ok())
        .collect()
}

async fn request(calls: &Calls, before: usize, method: &str) -> glib::Variant {
    until(method, || calls.borrow().len() > before).await;
    assert_eq!(calls.borrow().len(), before + 1);
    let (actual, value) = calls.borrow()[before].clone();
    assert_eq!(actual, method);
    value
}

#[test]
#[ignore = "requires a graphical display and dbus-daemon; uses isolated settings and a fake daemon"]
fn native_dialog_flows() {
    let Ok(address) = std::env::var("VEGA_DIALOG_TEST_BUS") else {
        isolated_child("application::dialog_tests::native_dialog_flows");
        return;
    };
    adw::init().unwrap();
    let context = glib::MainContext::default();
    context.block_on(async {
        let calls: Calls = Rc::new(RefCell::new(Vec::new()));
        let _service = fixture(&address, &calls);
        let dbus = VegaDbus::connect().await.unwrap();
        let shell = VegaShell::new();
        let window = adw::ApplicationWindow::builder().content(&shell.root).build();
        configure_backup(&shell, dbus.clone());
        configure_snapshots(&shell, dbus.clone());
        configure_network(&shell, &window, dbus.clone());
        connect_add_repo(&shell.software, &dbus, &shell.dashboard_updates);
        until("initial reads", || shell.network.status.text().contains("fixture rejected")).await;
        until("snapshot availability", || shell.snapshots.create.is_sensitive()).await;

        for enabled in [true, false] {
            crate::preferences::save(&crate::preferences::Settings {
                confirm_actions: enabled, ..Default::default()
            });
            assert_eq!(crate::preferences::confirmations_enabled(), enabled);
            println!("Testing real GTK flows, confirm_actions={enabled}");

            // Cancel, close and invalid input must not create a backup.
            for response in ["cancel", "close", "create"] {
                let before = calls.borrow().len();
                shell.backup.new_config.emit_clicked();
                let d = dialog(&calls, before).await;
                if response == "close" {
                    d.close();
                    until("close treated as cancel", || !d.is_mapped()).await;
                } else {
                    respond(&d, response).await;
                }
                glib::timeout_future(Duration::from_millis(30)).await;
                assert_eq!(calls.borrow().len(), before);
                if response == "create" {
                    assert!(shell.backup.status.text().contains("obrigatórios"));
                }
            }
            let before = calls.borrow().len();
            shell.backup.new_config.emit_clicked();
            let d = dialog(&calls, before).await;
            for (entry, text) in entries(&d).iter().zip([" documents ", " /tmp/a, /tmp/b, ", " /tmp/dest ", " test-uuid "]) {
                entry.set_text(text);
            }
            descendants(&d).into_iter().find_map(|w| w.downcast::<gtk::DropDown>().ok()).unwrap().set_selected(2);
            respond(&d, "create").await;
            assert_eq!(request(&calls, before, "CreateConfig").await,
                (("documents", vec!["/tmp/a", "/tmp/b"], "/tmp/dest", "test-uuid", "weekly"),).to_variant());
            until("backup request finished", || shell.backup.new_config.is_sensitive()).await;

            let before = calls.borrow().len();
            shell.snapshots.create.emit_clicked();
            let d = dialog(&calls, before).await;
            entries(&d)[0].set_text(" before update ");
            respond(&d, "create").await;
            assert_eq!(request(&calls, before, "CreateSnapshot").await, ("before update",).to_variant());
            until("snapshot request finished", || shell.snapshots.create.is_sensitive()).await;

            shell.network.show_interfaces(&[lyra_vega_dbus::NetworkInterface {
                name: "test-connection".into(), kind: "ethernet".into(), state: "connected".into(),
                ipv4: String::new(), ipv6: String::new(), gateway: String::new(), dns: String::new(),
                mac: String::new(), speed: String::new(), ssid: String::new(), signal: 0,
                device: "test0".into(), autoconf: true,
            }]);
            shell.network.interfaces.select_row(shell.network.interfaces.row_at_index(0).as_ref());
            let before = calls.borrow().len();
            shell.network.interface_action.emit_clicked();
            let d = dialog(&calls, before).await;
            let fields = entries(&d);
            assert_eq!(fields[0].text(), "test-connection");
            fields[1].set_text("192.0.2.2/24");
            fields[2].set_text("192.0.2.1");
            fields[3].set_text("192.0.2.53");
            respond(&d, "apply").await;
            assert_eq!(request(&calls, before, "SetStaticIpv4").await,
                ("test-connection", "192.0.2.2/24", "192.0.2.1", "192.0.2.53").to_variant());
            until("IPv4 error reported", || shell.network.status.text().contains("fixture rejected")).await;

            shell.network.show_wifi(&[lyra_vega_dbus::WifiNetwork {
                ssid: "test-wifi".into(), security: "WPA2".into(), signal: 80,
                active: false, device: "test0".into(),
            }]);
            let before = calls.borrow().len();
            descendants(&shell.network.wifi).into_iter()
                .find(|w| w.has_css_class("wifi-row-action")).unwrap()
                .downcast::<gtk::Button>().unwrap().emit_clicked();
            let d = dialog(&calls, before).await;
            d.extra_child().unwrap().downcast::<gtk::PasswordEntry>().unwrap().set_text("fixture-password");
            respond(&d, "confirm").await;
            assert_eq!(request(&calls, before, "ConnectWifi").await,
                ("test-wifi", "fixture-password").to_variant());

            let before = calls.borrow().len();
            shell.software.add_repo_name.set_text("");
            shell.software.add_repo_url.set_text("");
            shell.software.add_repo_button.emit_clicked();
            assert_eq!(calls.borrow().len(), before);
            shell.software.add_repo_name.set_text(" test-repo ");
            shell.software.add_repo_url.set_text(" https://example.invalid/repo ");
            assert!(shell.software.add_repo_button.is_sensitive());
            shell.software.add_repo_button.emit_clicked();
            assert_eq!(request(&calls, before, "AddRepo").await,
                ("test-repo", "https://example.invalid/repo").to_variant());

            // All supported AI mutations still require an explicit decision.
            for (name, method) in [("install_package", "Install"), ("remove_package", "Remove"), ("clear_package_cache", "ClearCache")] {
                for approved in [false, true] {
                    let before = calls.borrow().len();
                    let page = shell.assistant.clone();
                    let dbus = dbus.clone();
                    let task = context.spawn_local(async move {
                        handle_assistant_mutation(&page, &dbus, &crate::assistant::ToolCall {
                            name: name.into(), input: serde_json::json!({"origin":"official", "id":"fixture-package"}),
                        }).await;
                    });
                    let d = dialog(&calls, before).await;
                    respond(&d, if approved {"confirm"} else {"cancel"}).await;
                    task.await.unwrap();
                    if approved {
                        request(&calls, before, method).await;
                    } else {
                        assert_eq!(calls.borrow().len(), before);
                    }
                }
            }

            // Rollback review shows the daemon's differences before applying.
            shell.snapshots.show_snapshots(vec![lyra_vega_dbus::Snapshot {
                id: 7, timestamp: 0, trigger: "manual".into(), description: "fixture".into(),
            }]);
            for approved in [false, true] {
                let before = calls.borrow().len();
                let button = descendants(&shell.snapshots.list).into_iter()
                    .filter_map(|w| w.downcast::<gtk::Button>().ok())
                    .find(|b| b.label().as_deref() == Some("Aplicar")).unwrap();
                button.emit_clicked();
                let d = dialog(&calls, before).await;
                let preview = descendants(&d).into_iter().find_map(|w| w.downcast::<gtk::TextView>().ok()).unwrap();
                let buffer = preview.buffer();
                assert_eq!(buffer.text(&buffer.start_iter(), &buffer.end_iter(), false), "fixture-package: 2 -> 1");
                respond(&d, if approved {"rollback"} else {"cancel"}).await;
                until("rollback finished", || button.is_sensitive()).await;
                if approved {
                    assert_eq!(request(&calls, before, "Rollback").await, (7u32,).to_variant());
                } else {
                    assert_eq!(calls.borrow().len(), before);
                }
            }
            // Trust decisions cannot be inherited from optional confirmations.
            for key_id in ["fixture-key", ""] {
                for approved in [false, true] {
                    let before = calls.borrow().len();
                    let page = shell.software.clone();
                    let label = shell.dashboard_updates.clone();
                    let client = dbus.software();
                    let task = context.spawn_local(async move {
                        confirm_and_trust_repo_key(&page, &client, RepositoryKeyInfo {
                            transaction_id: 1, repo: "fixture-repo".into(), key_id: key_id.into(),
                            fingerprint: "ABCD 1234".into(), user_id: "Fixture signer".into(),
                        }, &label).await;
                    });
                    let d = dialog(&calls, before).await;
                    assert!(d.body().contains(if key_id.is_empty() { "sem verificação de assinatura" } else { "ABCD 1234" }));
                    respond(&d, if approved {"confirm"} else {"cancel"}).await;
                    task.await.unwrap();
                    if approved {
                        assert_eq!(request(&calls, before, "TrustRepoKey").await, ("fixture-repo", key_id).to_variant());
                    } else {
                        assert_eq!(calls.borrow().len(), before);
                    }
                }
            }

            // Plain confirmations still honor the preference.
            let d = adw::AlertDialog::new(Some("Already specified action"), None);
            d.add_responses(&[("cancel", "Cancel"), ("confirm", "Continue")]);
            d.set_close_response("cancel");
            let copy = d.clone();
            let task = context.spawn_local(async move { confirm_dialog(&copy, "confirm").await });
            if enabled {
                until("optional confirmation enabled", || d.is_mapped()).await;
                respond(&d, "cancel").await;
                assert!(!task.await.unwrap());
            } else {
                assert!(task.await.unwrap());
                assert!(!d.is_mapped());
            }
        }
        window.destroy();
    });
}

fn ipv4_interface(name: &str) -> lyra_vega_dbus::NetworkInterface {
    lyra_vega_dbus::NetworkInterface {
        name: name.into(),
        kind: "ethernet".into(),
        state: "connected".into(),
        ipv4: String::new(),
        ipv6: String::new(),
        gateway: String::new(),
        dns: String::new(),
        mac: String::new(),
        speed: String::new(),
        ssid: String::new(),
        signal: 0,
        device: "test0".into(),
        autoconf: true,
    }
}

async fn submit_ipv4(
    page: &crate::ui::NetworkPage,
    calls: &Calls,
    connection: &str,
    address: &str,
) {
    assert!(
        page.interface_action.is_sensitive(),
        "IPv4 action must allow retry"
    );
    let before = calls.borrow().len();
    page.interface_action.emit_clicked();
    let d = dialog(calls, before).await;
    let fields = entries(&d);
    assert_eq!(fields[0].text(), connection);
    fields[1].set_text(address);
    fields[2].set_text("192.0.2.1");
    fields[3].set_text("192.0.2.53");
    respond(&d, "apply").await;
    assert_eq!(
        request(calls, before, "SetStaticIpv4").await,
        (connection, address, "192.0.2.1", "192.0.2.53").to_variant()
    );
    assert!(
        !page.interface_action.is_sensitive(),
        "pending request must disable action"
    );
}

#[test]
#[ignore = "requires a graphical display and dbus-daemon; uses a private bus without host network changes"]
fn native_ipv4_retry() {
    let Ok(address) = std::env::var("VEGA_DIALOG_TEST_BUS") else {
        isolated_child("application::dialog_tests::native_ipv4_retry");
        return;
    };
    adw::init().unwrap();
    let context = glib::MainContext::default();
    context.block_on(async {
        let calls: Calls = Rc::new(RefCell::new(Vec::new()));
        let network = NetworkFixture::default();
        let _service = fixture_with_network(&address, &calls, Some(network.clone()));
        let dbus = VegaDbus::connect().await.unwrap();
        let shell = VegaShell::new();
        let window = adw::ApplicationWindow::builder()
            .content(&shell.root)
            .build();
        configure_network(&shell, &window, dbus);
        let page = &shell.network;
        until("initial fixture read rejected", || {
            page.status.text().contains("fixture rejected")
        })
        .await;
        let items = [
            ipv4_interface("test-connection"),
            ipv4_interface("other-connection"),
        ];
        for enabled in [true, false] {
            println!("Testing IPv4 retry and completion, confirm_actions={enabled}");
            crate::preferences::save(&crate::preferences::Settings {
                confirm_actions: enabled,
                ..Default::default()
            });
            page.show_interfaces(&items);
            assert!(!page.interface_action.is_sensitive());
            page.interfaces
                .select_row(page.interfaces.row_at_index(0).as_ref());

            // Rejection must permit a corrected request on the same selection.
            submit_ipv4(page, &calls, "test-connection", "192.0.2.2/24").await;
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_dbus_error(
                    "org.freedesktop.DBus.Error.AccessDenied",
                    "IPv4 request denied by fixture",
                );
            until("backend denial displayed", || {
                page.status
                    .text()
                    .contains("IPv4 request denied by fixture")
            })
            .await;
            assert!(
                page.interface_action.is_sensitive(),
                "denial left IPv4 action disabled"
            );
            assert_eq!(page.selected_interface().unwrap().name, "test-connection");

            // Success followed by a failed refresh also releases the button.
            *network.list_reply.borrow_mut() = None;
            submit_ipv4(page, &calls, "test-connection", "192.0.2.3/24").await;
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_value(None);
            until("refresh failure displayed", || {
                page.status.text().contains("fixture rejected")
            })
            .await;
            assert!(
                page.interface_action.is_sensitive(),
                "refresh failure blocked retry"
            );

            // A successful refresh replaces rows: availability follows the new selection.
            *network.list_reply.borrow_mut() = Some(
                (vec![(
                    "refreshed-connection",
                    "ethernet",
                    "connected",
                    "192.0.2.4/24",
                    "",
                    "192.0.2.1",
                    "192.0.2.53",
                    "",
                    "",
                    "",
                    0u32,
                    "test0",
                    false,
                )],)
                    .to_variant(),
            );
            submit_ipv4(page, &calls, "test-connection", "192.0.2.4/24").await;
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_value(None);
            until("interfaces refreshed", || {
                page.status.text() == "Interfaces de rede atualizadas"
            })
            .await;
            assert!(page.selected_interface().is_none());
            assert!(!page.interface_action.is_sensitive());
            page.interfaces
                .select_row(page.interfaces.row_at_index(0).as_ref());
            assert_eq!(
                page.selected_interface().unwrap().name,
                "refreshed-connection"
            );
            assert!(page.interface_action.is_sensitive());

            // Changing selection while applying cannot reopen a form or overlap requests.
            page.show_interfaces(&items);
            page.interfaces
                .select_row(page.interfaces.row_at_index(0).as_ref());
            submit_ipv4(page, &calls, "test-connection", "192.0.2.5/24").await;
            page.interfaces
                .select_row(page.interfaces.row_at_index(1).as_ref());
            assert!(!page.interface_action.is_sensitive());
            let before = calls.borrow().len();
            page.interface_action.emit_clicked();
            glib::timeout_future(Duration::from_millis(30)).await;
            assert!(active_dialog().is_none());
            assert_eq!(calls.borrow().len(), before);
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_dbus_error(
                    "org.lyraos.Test.Rejected",
                    "selection changed during request",
                );
            until("changed selection error displayed", || {
                page.status
                    .text()
                    .contains("selection changed during request")
            })
            .await;
            assert!(page.interface_action.is_sensitive());
            assert_eq!(page.selected_interface().unwrap().name, "other-connection");

            // Clearing selection while the backend runs must keep the button disabled.
            submit_ipv4(page, &calls, "other-connection", "192.0.2.6/24").await;
            page.interfaces.unselect_all();
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_dbus_error("org.lyraos.Test.Rejected", "no current selection");
            until("unselected error displayed", || {
                page.status.text().contains("no current selection")
            })
            .await;
            assert!(!page.interface_action.is_sensitive());
            page.interfaces
                .select_row(page.interfaces.row_at_index(1).as_ref());
            assert!(page.interface_action.is_sensitive());

            // A connection removed by a successful refresh cannot be configured again.
            *network.list_reply.borrow_mut() =
                Some(glib::Variant::parse(None, "(@a(ssssssssssusb) [],)").unwrap());
            submit_ipv4(page, &calls, "other-connection", "192.0.2.7/24").await;
            network
                .pending
                .borrow_mut()
                .pop()
                .unwrap()
                .return_value(None);
            until("empty interfaces refreshed", || {
                page.status.text() == "Interfaces de rede atualizadas"
            })
            .await;
            assert!(page.selected_interface().is_none());
            assert!(!page.interface_action.is_sensitive());
            assert!(network.pending.borrow().is_empty());
        }
        window.destroy();
    });
}

#[test]
#[ignore = "requires a disposable graphical compositor and private D-Bus"]
fn native_nvidia_flow() {
    let Ok(address) = std::env::var("VEGA_DIALOG_TEST_BUS") else {
        isolated_child("application::dialog_tests::native_nvidia_flow");
        return;
    };
    adw::init().unwrap();
    glib::MainContext::default().block_on(async {
        let calls: Calls = Rc::new(RefCell::new(Vec::new()));
        let nvidia = NvidiaFixture {
            state: Rc::new(RefCell::new("available".into())),
            capable: Rc::new(Cell::new(true)),
            result: Rc::new(Cell::new(0)),
        };
        let _service = fixture_with_services(&address, &calls, None, Some(nvidia.clone()));
        let dbus = VegaDbus::connect().await.unwrap();
        let card = crate::ui::NvidiaCard::new();
        let window = adw::ApplicationWindow::builder()
            .content(&card.root)
            .build();
        window.present();
        crate::nvidia::configure(&card, &window, dbus);
        until("public initial status", || !card.busy.get()).await;
        assert!(card.install.is_sensitive());
        assert!(calls.borrow().is_empty(), "startup mutated state");
        for optional in [true, false] {
            crate::preferences::save(&crate::preferences::Settings {
                confirm_actions: optional,
                ..Default::default()
            });
            for close in [false, true] {
                card.install.emit_clicked();
                let d = dialog(&calls, 0).await;
                assert_eq!(d.default_response().as_deref(), Some("cancel"));
                if close {
                    d.force_close();
                } else {
                    respond(&d, "cancel").await;
                }
                until("cancelled", || !card.busy.get()).await;
                assert!(
                    calls.borrow().is_empty(),
                    "cancel reached administrative method"
                );
            }
        }
        card.install.emit_clicked();
        respond(&dialog(&calls, 0).await, "install").await;
        until("denied authorization", || !card.busy.get()).await;
        assert_eq!(
            calls.borrow()[0],
            ("InstallNvidia".into(), (true,).to_variant())
        );
        assert!(card.install.is_sensitive(), "denial cannot leave UI busy");
        calls.borrow_mut().clear();
        for state in [
            "conflict",
            "inconsistent",
            "kernel-missing",
            "unknown-secure-boot",
            "active",
            "future-state",
        ] {
            *nvidia.state.borrow_mut() = state.into();
            card.refresh.emit_clicked();
            until("blocked state", || !card.busy.get()).await;
            assert!(!card.install.is_sensitive(), "{state}");
            assert!(calls.borrow().is_empty());
        }
        for result in [1u8, 2, 3] {
            *nvidia.state.borrow_mut() = "available".into();
            nvidia.result.set(result);
            card.refresh.emit_clicked();
            until("ready", || !card.busy.get()).await;
            assert!(card.install.is_sensitive());
            let before = calls.borrow().len();
            card.install.emit_clicked();
            respond(&dialog(&calls, before).await, "install").await;
            until("transaction completed/lost owner", || !card.busy.get()).await;
            assert!(
                !card.install.is_sensitive(),
                "completed/failed/unknown state must be refreshed"
            );
            assert_eq!(calls.borrow().len(), before + 1);
        }
        window.close();
    });
}

#[test]
#[ignore = "requires a graphical display and dbus-daemon; uses isolated settings and a fake daemon"]
fn native_dashboard_refresh() {
    let Ok(address) = std::env::var("VEGA_DIALOG_TEST_BUS") else {
        isolated_child("application::dialog_tests::native_dashboard_refresh");
        return;
    };
    adw::init().unwrap();
    glib::MainContext::default().block_on(async {
        let calls: Calls = Rc::default();
        let service = fixture(&address, &calls);
        let pending: Rc<RefCell<Vec<gio::DBusMethodInvocation>>> = Rc::default();
        let pings = Rc::new(Cell::new(0));
        let xml = gio::DBusNodeInfo::for_xml(r#"<node><interface name="org.lyraos.Vega1.System">
            <method name="Ping"><arg type="b" direction="out"/></method>
            <method name="Version"><arg type="s" direction="out"/></method>
            <method name="Distro"><arg type="s" direction="out"/></method>
            <method name="Logo"><arg type="s" direction="out"/></method>
            <method name="DiskUsage"><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="u" direction="out"/></method>
            </interface></node>"#).unwrap();
        service.register_object("/org/lyraos/Vega1", &xml.interfaces()[0])
            .method_call({
                let pending = pending.clone();
                let pings = pings.clone();
                move |_, _, _, _, method, _, invocation| {
                    if method == "Ping" {
                        pings.set(pings.get() + 1);
                        pending.borrow_mut().push(invocation);
                        return;
                    }
                    let result = match method {
                        "Version" => ("fixture",).to_variant(),
                        "Distro" => ("Lyra test",).to_variant(),
                        "Logo" => ("",).to_variant(),
                        "DiskUsage" => ("10 GiB", "100 GiB", 10_u32).to_variant(),
                        _ => unreachable!(),
                    };
                    invocation.return_value(Some(&result));
                }
            }).build().unwrap();
        let shell = VegaShell::new();
        let window = adw::ApplicationWindow::builder().content(&shell.root).build();
        update_content(shell.clone(), window.clone());
        until("first summary", || !pending.borrow().is_empty()).await;
        for _ in 0..50 { shell.dashboard_button.emit_clicked(); }
        glib::timeout_future(Duration::from_millis(100)).await;
        assert_eq!(pings.get(), 1, "clicks must not start overlapping summaries");
        pending.borrow_mut().pop().unwrap().return_value(Some(&(true,).to_variant()));
        until("one coalesced followup", || !pending.borrow().is_empty()).await;
        assert_eq!(pings.get(), 2);
        pending.borrow_mut().pop().unwrap().return_dbus_error("org.lyraos.Test.Rejected", "status unavailable");
        until("status failure", || shell.backend_status.text().contains("status unavailable")).await;
        assert!(shell.dashboard_disk.text().contains("10%"), "status failure must not overwrite independent cards");
        assert!(!shell.dashboard_backup.text().contains("status unavailable"));
        glib::timeout_future(Duration::from_millis(100)).await;
        assert_eq!(pings.get(), 2, "the burst must not leave a queue");
        shell.stack.set_visible_child_name("software");
        shell.dashboard_button.emit_clicked();
        assert_eq!(shell.stack.visible_child_name().as_deref(), Some("dashboard"));
        until("return to dashboard", || !pending.borrow().is_empty()).await;
        pending.borrow_mut().pop().unwrap().return_value(Some(&(true,).to_variant()));
        until("recovery", || shell.backend_status.text().contains("Lyra test")).await;
        glib::timeout_future(Duration::from_millis(100)).await;
        shell.dashboard_button.emit_clicked();
        until("click already active dashboard", || !pending.borrow().is_empty()).await;
        assert_eq!(pings.get(), 4);
        pending.borrow_mut().pop().unwrap().return_value(Some(&(true,).to_variant()));
        window.close();
    });
}

#[test]
#[ignore = "requires a graphical display and dbus-daemon; uses isolated settings and a fake daemon"]
fn native_dashboard_updates() {
    let Ok(address) = std::env::var("VEGA_DIALOG_TEST_BUS") else {
        isolated_child("application::dialog_tests::native_dashboard_updates");
        return;
    };
    adw::init().unwrap();
    glib::MainContext::default().block_on(async {
        let pending: PendingUpdates = Rc::default();
        let service =
            fixture_with_updates(&address, &Rc::default(), None, None, Some(pending.clone()));
        let dbus = VegaDbus::connect().await.unwrap();
        let card = crate::refresh::UpdatesCard::new(gtk::Label::new(None));
        let first = {
            let card = card.clone();
            let dbus = dbus.clone();
            glib::MainContext::default().spawn_local(async move {
                refresh_dashboard_updates(&card, &dbus.software()).await;
            })
        };
        until("initial updates query", || !pending.borrow().is_empty()).await;
        for _ in 0..50 {
            refresh_dashboard_updates(&card, &dbus.software()).await;
        }
        assert_eq!(pending.borrow().len(), 1);
        pending
            .borrow_mut()
            .pop()
            .unwrap()
            .return_dbus_error("org.lyraos.Test.Rejected", "updates unavailable");
        until("followup after failed updates query", || {
            !pending.borrow().is_empty()
        })
        .await;
        assert_eq!(pending.borrow().len(), 1);
        pending.borrow_mut().pop().unwrap().return_value(Some(
            &glib::Variant::parse(None, "(@a(ssssbss) [],)").unwrap(),
        ));
        first.await.unwrap();
        assert_eq!(card.text(), gettext("Tudo em dia"));
        assert!(pending.borrow().is_empty());
        // A later request must still be accepted after the failed query and burst.
        let next = {
            let card = card.clone();
            glib::MainContext::default().spawn_local(async move {
                refresh_dashboard_updates(&card, &dbus.software()).await;
            })
        };
        until("subsequent updates query", || !pending.borrow().is_empty()).await;
        pending
            .borrow_mut()
            .pop()
            .unwrap()
            .return_dbus_error("org.lyraos.Test.Rejected", "updates unavailable");
        next.await.unwrap();
        assert!(card.text().contains("updates unavailable"));
        drop(service);
    });
}
