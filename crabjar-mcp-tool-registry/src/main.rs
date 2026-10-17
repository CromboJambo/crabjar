use crabjar_mcp_tool_registry::ToolRegistryMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = ToolRegistryMcpServer::new()?;

    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;

    // Wait for the service loop to terminate (e.g. when stdin closes)
    running.waiting().await?;

    Ok(())
}
