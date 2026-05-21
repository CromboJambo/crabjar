use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::Deserialize;
use serde_json::json;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use zed_acp_server::{AcpAgentServer, AcpResponse, ZedRequest};

#[derive(Debug, Deserialize)]
struct AcpRequest {
    method: String,
    params: serde_json::Value,
}

fn parse_request(line: &str) -> Result<AcpRequest, anyhow::Error> {
    serde_json::from_str(line)
        .map_err(|e| anyhow::anyhow!("Failed to parse request: {}", e))
}

fn map_method(request: AcpRequest) -> Result<ZedRequest, anyhow::Error> {
    let method = request.method.as_str();
    let params = request.params;

    match method {
        "session/new" => {
            let cwd = params
                .get("cwd")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing cwd in session/new"))?;
            Ok(ZedRequest::NewSession { cwd: cwd.to_string() })
        }
        "session/load" => {
            let session_id = params
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing session_id in session/load"))?;
            let cwd = params
                .get("cwd")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing cwd in session/load"))?;
            Ok(ZedRequest::LoadSession {
                session_id: session_id.to_string(),
                cwd: cwd.to_string(),
            })
        }
        "session/close" => {
            let session_id = params
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing session_id in session/close"))?;
            Ok(ZedRequest::CloseSession {
                session_id: session_id.to_string(),
            })
        }
        "session/list" => Ok(ZedRequest::ListSessions),
        "prompt" => {
            let session_id = params
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing session_id in prompt"))?;
            let message = params
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing message in prompt"))?;
            Ok(ZedRequest::Prompt {
                session_id: session_id.to_string(),
                message: message.to_string(),
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
            Ok(ZedRequest::ToolCall {
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
            Ok(ZedRequest::Authenticate {
                auth_method: auth_method.to_string(),
            })
        }
        _ => Err(anyhow::anyhow!("unknown method: {}", method)),
    }
}

fn format_response(response: AcpResponse) -> serde_json::Value {
    match response {
        AcpResponse::Result { value } => json!({
            "type": "result",
            "value": value,
        }),
        AcpResponse::Error { message } => json!({
            "type": "error",
            "message": message,
        }),
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

    let server = AcpAgentServer::default();

    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read stdin: {}", e))?;

        if line.trim().is_empty() {
            continue;
        }

        let request = parse_request(&line)?;
        let zed_request = map_method(request)?;

        let response = server.handle_request(zed_request).await;
        let response_json = format_response(response);

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
