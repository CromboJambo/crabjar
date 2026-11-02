use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use zed_acp_server::{AcpAgentServer, AcpResponse, AcpRequest};

#[derive(Debug, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: Option<String>,
    id: Option<serde_json::Value>,
    method: Option<String>,
    params: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

fn parse_message(line: &str) -> Result<JsonRpcMessage, anyhow::Error> {
    serde_json::from_str(line).map_err(|e| anyhow::anyhow!("Failed to parse JSON-RPC message: {}", e))
}

fn map_method(id: serde_json::Value, method: &str, params: serde_json::Value) -> Result<AcpRequest, anyhow::Error> {
    match method {
        "initialize" => {
            let protocol_version = params
                .get("protocolVersion")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(1);
            let client_capabilities = params.get("clientCapabilities").cloned().unwrap_or(json!({}));
            let client_info = params.get("clientInfo").cloned();
            Ok(AcpRequest::Initialize {
                id,
                protocol_version,
                client_capabilities,
                client_info,
            })
        }
        "session/new" => {
            let cwd = params
                .get("cwd")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing cwd in session/new"))?;
            let mcp_servers = params
                .get("mcpServers")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(AcpRequest::SessionNew {
                id,
                cwd: cwd.to_string(),
                mcp_servers,
            })
        }
        "session/load" => {
            let session_id = params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing sessionId in session/load"))?;
            let cwd = params
                .get("cwd")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing cwd in session/load"))?;
            Ok(AcpRequest::SessionLoad {
                id,
                session_id: session_id.to_string(),
                cwd: cwd.to_string(),
            })
        }
        "session/close" => {
            let session_id = params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing sessionId in session/close"))?;
            Ok(AcpRequest::SessionClose {
                id,
                session_id: session_id.to_string(),
            })
        }
        "session/list" => Ok(AcpRequest::SessionList { id }),
        "session/prompt" => {
            let session_id = params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing sessionId in session/prompt"))?;
            let prompt = params
                .get("prompt")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(AcpRequest::SessionPrompt {
                id,
                session_id: session_id.to_string(),
                prompt,
            })
        }
        "tool_call" => {
            let session_id = params
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing session_id in tool_call"))?;
            let function_name = params
                .get("function_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing function_name in tool_call"))?;
            let arguments = params
                .get("arguments")
                .ok_or_else(|| anyhow::anyhow!("missing arguments in tool_call"))?;
            Ok(AcpRequest::ToolCall {
                id,
                session_id: session_id.to_string(),
                function_name: function_name.to_string(),
                arguments: arguments.clone(),
            })
        }
        "authenticate" => {
            let auth_method = params
                .get("auth_method")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing auth_method in authenticate"))?;
            Ok(AcpRequest::Authenticate {
                id,
                auth_method: auth_method.to_string(),
            })
        }
        _ => Err(anyhow::anyhow!("unknown method: {}", method)),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    info!("ACP agent server listening on stdin/stdout");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    let mut server = AcpAgentServer::default();

    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read stdin: {}", e))?;

        if line.trim().is_empty() {
            continue;
        }

        let msg = parse_message(&line)?;

        // Handle responses (id present, no method) - ignore from client
        if msg.id.is_some() && msg.method.is_none() {
            info!("Received client response: {:?}", msg.id);
            continue;
        }

        // Handle notifications (no id, has method) - log but don't respond
        if msg.id.is_none() && msg.method.is_some() {
            info!("Received notification: {}", msg.method.as_ref().unwrap());
            continue;
        }

        // Handle requests (id present, has method)
        let Some(id) = msg.id else {
            continue;
        };
        let Some(method) = msg.method else {
            anyhow::bail!("message has id but no method");
        };
        let params = msg.params.unwrap_or(json!({}));

        let acp_request = map_method(id.clone(), &method, params)?;
        let response = server.handle_request(acp_request).await;
        let response_json = match response {
            Ok(AcpResponse::Result { id, result }) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Ok(AcpResponse::Error { id, code, message }) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": code,
                    "message": message
                }
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32603,
                    "message": format!("ACP server error: {}", e)
                }
            }),
        };

        let output = serde_json::to_string(&response_json)
            .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;

        writer
            .write_all(output.as_bytes())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write stdout: {}", e))?;

        writer
            .write_all(b"\n")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write stdout: {}", e))?;

        writer
            .flush()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to flush stdout: {}", e))?;
    }
}