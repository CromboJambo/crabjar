use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use zed_acp_server::AcpAgentServer;

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

        let request = serde_json::from_str(&line)
            .map_err(|e| anyhow::anyhow!("Failed to parse request: {}", e))?;

        let response = server.handle_request(request).await;

        let response_json = serde_json::to_string(&response)
            .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;

        writer
            .write_all(response_json.as_bytes())
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
