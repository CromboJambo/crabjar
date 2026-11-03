use analyze_mcp_server::AnalyzeMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = AnalyzeMcpServer;

    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;

    // Wait for the service loop to terminate (e.g. when stdin closes)
    running.waiting().await?;

    Ok(())
}