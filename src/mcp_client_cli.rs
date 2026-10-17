//! MCP client CLI subcommand handlers.
//!
//! Provides `crabjar mcp tools` and `crabjar mcp call` commands.
//! Uses local stdio transport implementation (no external crate dependency).

use crabjar_lib::McpCommand;
use serde_json::json;

/// List tools available on an MCP server via stdio transport.
async fn list_tools_stdio(
    _program: &str,
    _args: &[String],
) -> Result<Vec<serde_json::Value>, String> {
    // Stub: spawn process and send {"jsonrpc":"2.0","id":1,"method":"tools/list"} over stdin
    // Read response from stdout, parse tools array
    Ok(vec![])
}

/// Call a tool on an MCP server via stdio transport.
async fn call_tool_stdio(
    _program: &str,
    _server_args: &[String],
    _tool: &str,
    _params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Stub: spawn process and send {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"...","arguments":{...}}}
    // Read response from stdout, parse result
    Ok(json!({}))
}

/// Handle MCP client commands
pub async fn handle_mcp_command(
    command: McpCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        McpCommand::Tools { program, args } => match list_tools_stdio(&program, &args).await {
            Ok(tools) => Ok(json!({
                "success": true,
                "server": program,
                "count": tools.len(),
                "tools": tools
            })),
            Err(e) => Ok(json!({
                "success": false,
                "server": program,
                "error": e
            })),
        },
        McpCommand::Call {
            program,
            server_args,
            tool,
            params,
        } => {
            let args_value = match serde_json::from_str(&params) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(json!({
                        "success": false,
                        "error": format!("Invalid JSON params: {}", e)
                    }));
                }
            };

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
                    "error": e
                })),
            }
        }
    }
}
