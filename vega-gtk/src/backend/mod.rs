//! Ponte entre a interface web e o vegad.
//!
//! Cada submódulo expõe comandos Tauri finos sobre um cliente do
//! `lyra-vega-dbus`. Os comandos devolvem DTOs serializáveis e erros já
//! traduzidos (o `Display` dos clientes usa gettext), então a interface só
//! exibe o texto. Operações longas devolvem o id da transação na hora e
//! repassam o andamento como eventos.

pub mod backup;
pub mod bluetooth;
pub mod datetime;
pub mod diagnostics;
pub mod flatpak_update;
pub mod gpu_bench;
pub mod hardware;
pub mod kernel;
pub mod logs;
pub mod monitor;
pub mod network;
pub mod notify;
pub mod preparation;
pub mod services;
pub mod snapshots;
pub mod software;
pub mod storage;
pub mod system;
pub mod users;

use async_lock::Mutex;
use lyra_vega_dbus::VegaDbus;

/// Conexão única com o system bus, criada sob demanda. Uma falha não fica
/// guardada: a próxima chamada tenta de novo, como o refresh do vega-gtk.
#[derive(Default)]
pub struct Backend {
    dbus: Mutex<Option<VegaDbus>>,
}

impl Backend {
    pub async fn dbus(&self) -> Result<VegaDbus, String> {
        let mut slot = self.dbus.lock().await;
        if let Some(dbus) = slot.as_ref() {
            return Ok(dbus.clone());
        }
        let dbus = VegaDbus::connect()
            .await
            .map_err(|error| error.to_string())?;
        *slot = Some(dbus.clone());
        Ok(dbus)
    }
}

pub type State<'a> = tauri::State<'a, Backend>;
pub type CmdResult<T> = Result<T, String>;

pub fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Gera um DTO serializável com os mesmos campos do tipo do cliente.
macro_rules! dto {
    ($name:ident from $source:path { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name { $(pub $field: $ty),* }

        impl From<$source> for $name {
            fn from(value: $source) -> Self {
                Self { $($field: value.$field.into()),* }
            }
        }
    };
}
pub(crate) use dto;

pub fn list<S, T: From<S>>(items: Vec<S>) -> Vec<T> {
    items.into_iter().map(T::from).collect()
}
