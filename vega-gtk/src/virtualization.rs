//! Máquinas virtuais via libvirt (crate vega-virtualization), sem vegad.

use std::sync::atomic::AtomicBool;

use serde_json::Value;

/// Pedido de cancelamento da criação em andamento.
#[derive(Default)]
pub struct Creation {
    pub cancel: AtomicBool,
}

#[tauri::command]
pub async fn vms_call(op: String, args: Value) -> Result<Value, String> {
    let _ = args;
    Err(format!("operação de máquinas virtuais desconhecida: {op}"))
}
