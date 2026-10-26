//! crabjar-compress-mcp: MCP server wrapping crabjar-compress for Hermes integration.
//!
//! Exposes content-aware compression as an MCP tool that can replace Hermes'
//! default 'compressor' tool with zero-LLM, deterministic compression.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use crabjar_compress::{compress_tool_result, CompressionConfig, CompressionResult};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompressRequest {
    /// The text content to compress.
    pub content: String,
    /// Optional maximum tokens after compression (default: 500).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_max_tokens() -> usize {
    500
}

#[derive(Debug, Clone)]
pub struct CompressServer;

#[tool_router(server_handler)]
impl CompressServer {
    #[tool(description = "Compress tool output using content-aware strategies. Detects JSON, search results, logs, diffs, HTML, or prose and applies appropriate compression. Returns compressed text with metadata about reduction achieved.")]
    fn compress(&self, params: Parameters<CompressRequest>) -> String {
        let req = params.0;

        let config = CompressionConfig {
            max_tokens: req.max_tokens,
            ..Default::default()
        };

        let result: CompressionResult = compress_tool_result(&req.content, &config);

        serde_json::to_string(&serde_json::json!({
            "compressed": result.compressed,
            "original_chars": result.original_chars,
            "compressed_chars": result.compressed_chars,
            "reduction_percent": result.reduction_percent,
            "content_type": format!("{:?}", result.content_type)
        })).unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {}\"}}", e))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = CompressServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
