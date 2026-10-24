//! Comandos do assistente de IA para a interface web.

use serde_json::Value;

#[tauri::command]
pub async fn assistant_call(op: String, args: Value) -> Result<Value, String> {
    let _ = args;
    Err(format!("operação do assistente desconhecida: {op}"))
}
