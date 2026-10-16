//! MCP client CLI subcommand handlers.
//!
//! Provides `crabjar mcp tools` and `crabjar mcp call` commands.

use serde_json::json;
use crabjar_lib::McpCommand;
use crabjar_mcp_client::{list_tools_stdio, call_tool_stdio};

/// Handle MCP client commands
pub async fn handle_mcp_command(
    command: McpCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        McpCommand::Tools { program, args } => {
            // List tools available on an external MCP server via stdio transport
            match list_tools_stdio(&program, &args).await {
                Ok(tools) => Ok(json!({
                    "success": true,
                    "server": program,
                    "count": tools.len(),
                    "tools": tools
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "server": program,
                    "error": e.to_string()
                })),
            }
        }
        McpCommand::Call { program, server_args, tool, params } => {
            // Parse JSON params
            let args_value = match serde_json::from_str(&params) {
                Ok(v) => v,
                Err(e) => return Ok(json!({
                    "success": false,
                    "error": format!("Invalid JSON params: {}", e)
                })),
            };
            
            // Call the tool on an external MCP server via stdio transport
            match call_tool_stdio(&program, &server_args, &tool, args_value).await {
                Ok(result) => Ok(json!({
                    "success": true,
                    "server": program,
                    "tool": tool,
                    "result": result
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "server": program,
                    "tool": tool,
                    "error": e.to_string()
                })),
            }
        }
    }
}
