//! Comandos do assistente de IA para a interface web.
//!
//! A lógica (provedores, chaves no Secret Service, histórico 0600, limite
//! diário, auditoria, redação) continua em `assistant.rs`. Aqui ficam só a
//! ponte com a interface e as ferramentas: as de leitura rodam no backend e
//! já devolvem o resultado embrulhado como dado não confiável; as de mutação
//! só viram uma proposta validada, que a interface executa depois de uma
//! confirmação obrigatória.

use lyra_vega_dbus::{LogsClient, SoftwareClient, SystemClient};
use serde_json::{Value, json};

use super::{
    Attachment, Message, Provider, Settings, ToolCall, audit, clear_history, clear_key,
    continue_after_tool, install_origin_allowed, is_mutating_tool, keyring_available, list_models,
    load_history, load_settings, read_attachment, redact, save_history, save_key, save_settings,
    send,
};
use crate::backend::State;
use crate::i18n::gettext;

type Res = Result<Value, String>;

fn arg<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| format!("argumento inválido `{key}`: {error}"))
}

fn to_value(value: impl serde::Serialize) -> Res {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

/// Roda código bloqueante (HTTP, keyring, disco) fora do loop assíncrono.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, super::AssistantError> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| gettext("Falha interna ao consultar o provedor"))?
        .map_err(|error| error.to_string())
}

fn tool_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn tool_u32(value: &Value, key: &str, default: u32) -> u32 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(default)
}

#[tauri::command]
pub async fn assistant_call(state: State<'_>, op: String, args: Value) -> Res {
    match op.as_str() {
        // Estado inicial da página: configurações, histórico e keyring.
        "state" => {
            let (settings, history, keyring) =
                blocking(|| Ok((load_settings(), load_history(), keyring_available()))).await?;
            let providers: Vec<Value> = Provider::ALL
                .iter()
                .map(|p| json!({ "id": p.id(), "label": p.label(), "default_model": p.default_model() }))
                .collect();
            Ok(
                json!({ "settings": settings, "history": history, "keyring": keyring, "providers": providers }),
            )
        }
        "save_settings" => {
            let settings: Settings = arg(&args, "settings")?;
            blocking(move || save_settings(&settings)).await?;
            Ok(Value::Null)
        }
        "save_key" => {
            let provider: Provider = arg(&args, "provider")?;
            let key: String = arg(&args, "key")?;
            blocking(move || save_key(provider, &key)).await?;
            Ok(Value::Null)
        }
        "clear_key" => {
            let provider: Provider = arg(&args, "provider")?;
            blocking(move || clear_key(provider)).await?;
            Ok(Value::Null)
        }
        "list_models" => {
            let provider: Provider = arg(&args, "provider")?;
            to_value(blocking(move || list_models(provider)).await?)
        }
        "read_attachment" => {
            let path: String = arg(&args, "path")?;
            let attachment: Attachment =
                blocking(move || read_attachment(std::path::Path::new(&path))).await?;
            to_value(attachment)
        }
        "audit" => {
            let kind: String = arg(&args, "kind")?;
            let detail: String = arg(&args, "detail")?;
            blocking(move || audit(&kind, &detail)).await?;
            Ok(Value::Null)
        }
        "save_history" => {
            let messages: Vec<Message> = arg(&args, "messages")?;
            blocking(move || save_history(&messages)).await?;
            Ok(Value::Null)
        }
        "clear_history" => {
            blocking(clear_history).await?;
            Ok(Value::Null)
        }
        // Uma rodada com o provedor. A primeira conta no limite diário.
        "round" => {
            let settings: Settings = arg(&args, "settings")?;
            let history: Vec<Message> = arg(&args, "history")?;
            let first: bool = arg(&args, "first")?;
            let reply = blocking(move || {
                if first {
                    send(&settings, &history)
                } else {
                    continue_after_tool(&settings, &history)
                }
            })
            .await?;
            to_value(reply)
        }
        "tool" => {
            let call: ToolCall = ToolCall {
                name: arg(&args, "name")?,
                input: args.get("input").cloned().unwrap_or(Value::Null),
            };
            run_tool(&state, call).await
        }
        "mutation_plan" => {
            let call = ToolCall {
                name: arg(&args, "name")?,
                input: args.get("input").cloned().unwrap_or(Value::Null),
            };
            mutation_plan(call).await
        }
        "mutation_result" => {
            let description: String = arg(&args, "description")?;
            let outcome: String = arg(&args, "outcome")?;
            let detail: String = arg(&args, "detail").unwrap_or_default();
            let (kind, text) = match outcome.as_str() {
                "approved" => ("mutation_approved", description.clone()),
                "rejected" => ("mutation_rejected", description.clone()),
                _ => ("mutation_error", format!("{description}: {detail}")),
            };
            blocking(move || audit(kind, &text)).await?;
            Ok(Value::Null)
        }
        _ => Err(format!("operação do assistente desconhecida: {op}")),
    }
}

/// Ferramentas de leitura. Devolve a mensagem a acrescentar ao histórico:
/// `{ role, content }`, ou erro.
async fn run_tool(state: &State<'_>, call: ToolCall) -> Res {
    if is_mutating_tool(&call.name) {
        return Err(
            gettext("Ferramenta desconhecida recusada: {name}").replace("{name}", &call.name)
        );
    }
    let dbus = state.dbus().await?;
    let result: Result<String, String> = match call.name.as_str() {
        "search_packages" => {
            let query = tool_string(&call.input, "query");
            dbus.software()
                .search(&query)
                .await
                .map(|packages| {
                    packages
                        .into_iter()
                        .take(20)
                        .map(|p| format!("{} • {} • {}", p.id, p.origin, p.description))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .map_err(|e| e.to_string())
        }
        "list_available_updates" => dbus
            .software()
            .list_updates()
            .await
            .map(|packages| {
                packages
                    .into_iter()
                    .map(|p| format!("{} • {}", p.id, p.origin))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .map_err(|e| e.to_string()),
        "get_recent_logs" => {
            let unit = tool_string(&call.input, "unit");
            let priority = tool_string(&call.input, "priority");
            let max_lines = tool_u32(&call.input, "max_lines", 50).clamp(1, 200);
            dbus.logs()
                .query(&unit, &priority, "", "", max_lines)
                .await
                .map(|lines| {
                    let logs = lines.join("\n");
                    if crate::preferences::redact_ai_data() {
                        redact(&logs)
                    } else {
                        logs
                    }
                })
                .map_err(|e| e.to_string())
        }
        "get_system_status" => {
            let status = dbus.system().status().await.map_err(|e| e.to_string())?;
            let (used, total, percent) = dbus
                .system()
                .disk_usage()
                .await
                .map_err(|e| e.to_string())?;
            let _ = blocking(|| audit("read", "get_system_status concluída")).await;
            // Como no vega-gtk: vira uma mensagem do assistente, sem nova rodada.
            return Ok(json!({
                "role": "assistant",
                "content": gettext("Sistema: {distro} • vegad {version}\nDisco: {used} de {total} ({percent}%)")
                    .replace("{distro}", &status.distro)
                    .replace("{version}", &status.version)
                    .replace("{used}", &used)
                    .replace("{total}", &total)
                    .replace("{percent}", &percent.to_string()),
            }));
        }
        _ => {
            let name = call.name.clone();
            let _ = blocking(move || audit("tool_rejected", &name)).await;
            return Err(
                gettext("Ferramenta desconhecida recusada: {name}").replace("{name}", &call.name)
            );
        }
    };
    match result {
        Ok(output) => {
            let output = if output.is_empty() {
                gettext("Nenhum resultado.")
            } else {
                output
            };
            let instruction = gettext("Continue a resposta usando este resultado.");
            let detail = format!("{} concluída", call.name);
            let _ = blocking(move || audit("read", &detail)).await;
            Ok(json!({
                "role": "user",
                "content": format!(
                    "<dado_nao_confiavel origem=\"tool:{}\">\n{output}\n</dado_nao_confiavel>\n{instruction}",
                    call.name
                ),
            }))
        }
        Err(error) => {
            let detail = format!("{}: {error}", call.name);
            let _ = blocking(move || audit("read_error", &detail)).await;
            Err(error)
        }
    }
}

/// Valida uma proposta de mutação e devolve o que a interface precisa para
/// a confirmação obrigatória e a ação de software correspondente.
async fn mutation_plan(call: ToolCall) -> Res {
    if !is_mutating_tool(&call.name) {
        return Err(
            gettext("Ferramenta desconhecida recusada: {name}").replace("{name}", &call.name)
        );
    }
    let origin = tool_string(&call.input, "origin");
    let id = tool_string(&call.input, "id");
    if call.name != "clear_package_cache" && id.is_empty() {
        return Err(gettext("Proposta recusada: pacote sem identificador"));
    }
    if call.name == "install_package" && !install_origin_allowed(&origin) {
        return Err(gettext(
            "Esta origem não pode ser instalada pelo Assistente; use a tela Software para revisão",
        ));
    }
    let plan = match call.name.as_str() {
        "install_package" => json!({
            "title": gettext("Instalar pacote?"),
            "description": gettext("Instalar {id} da origem {origin}.").replace("{id}", &id).replace("{origin}", &origin),
            "confirm": gettext("Instalar"),
            "destructive": false,
            "action": { "kind": "install", "origin": origin, "id": id },
        }),
        "remove_package" => json!({
            "title": gettext("Remover pacote?"),
            "description": gettext("Remover {id} da origem {origin}.").replace("{id}", &id).replace("{origin}", &origin),
            "confirm": gettext("Remover"),
            "destructive": true,
            "action": { "kind": "remove", "origin": origin, "id": id },
        }),
        _ => json!({
            "title": gettext("Limpar cache?"),
            "description": gettext("Remover os pacotes baixados do cache."),
            "confirm": gettext("Limpar"),
            "destructive": true,
            "action": { "kind": "clear-cache" },
        }),
    };
    let description = plan["description"].as_str().unwrap_or_default().to_owned();
    let _ = blocking(move || audit("mutation_proposed", &description)).await;
    Ok(plan)
}
