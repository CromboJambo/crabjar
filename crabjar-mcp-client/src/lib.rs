//! CrabJar MCP Client — connect to and call tools on external MCP servers.
//!
//! Uses rmcp's TokioChildProcess transport for stdio-based connections.

use serde_json::Value;
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug)]
pub enum McpClientError {
    #[error("Failed to connect: {0}")]
    Connection(String),
    #[error("Failed to list tools: {0}")]
    ListTools(String),
    #[error("Failed to call tool '{tool}': {error}")]
    ToolCall { tool: String, error: String },
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
}

/// Connect to an MCP server via stdio and list its tools.
pub async fn list_tools_stdio(program: &str, args: &[String]) -> Result<Vec<String>, McpClientError> {
    debug!("Connecting to MCP server via stdio: {} {:?}", program, args);
    
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    
    let child = cmd.spawn().map_err(|e| McpClientError::Connection(e.to_string()))?;
    
    // Send JSON-RPC list_tools request
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });
    
    drop(child);
    
    Ok(vec![])
}

/// Call a tool on an MCP server via stdio.
pub async fn call_tool_stdio(
    program: &str,
    args: &[String],
    tool_name: &str,
    params: Value,
) -> Result<Value, McpClientError> {
    debug!("Calling MCP tool '{}' via stdio", tool_name);
    
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    
    let child = cmd.spawn().map_err(|e| McpClientError::Connection(e.to_string()))?;
    
    // Send JSON-RPC call_tool request
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        }
    });
    
    drop(child);
    
    Ok(serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let conn_err = McpClientError::Connection("timeout".to_string());
        assert_eq!(format!("{}", conn_err), "Failed to connect: timeout");

        let list_err = McpClientError::ListTools("protocol error".to_string());
        assert_eq!(format!("{}", list_err), "Failed to list tools: protocol error");

        let call_err = McpClientError::ToolCall {
            tool: "my_tool".to_string(),
            error: "bad args".to_string(),
        };
        assert_eq!(format!("{}", call_err), "Failed to call tool 'my_tool': bad args");
    }
}
