//! Plugin trait implementation for StructuredAuxPlugin.

use crate::plugin::StructuredAuxPlugin;
use async_trait::async_trait;
use crabjar_plugin::{Plugin, ToolDefinition, ToolResult};

#[async_trait]
impl Plugin for StructuredAuxPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: format!("{}.approval", self.name),
                description: "Bounded approval decision via JEV (approve/deny/needs_review)"
                    .to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "request": {"type": "string"},
                        "policy": {"type": "string"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.mcp", self.name),
                description: "Bounded MCP tool selection via JEV".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "request": {"type": "string"},
                        "tools": {"type": "array"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.risk", self.name),
                description: "Bounded risk assessment via JEV (low/medium/high/critical)"
                    .to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {"type": "string"},
                        "context": {"type": "string"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.priority", self.name),
                description: "Bounded priority ranking via JEV".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tasks": {"type": "array"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.retry", self.name),
                description: "Bounded retry decision via JEV (retry/give_up)".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "error": {"type": "string"},
                        "attempts": {"type": "integer"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.resource", self.name),
                description: "Bounded resource allocation via JEV".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "request": {"type": "string"},
                        "resources": {"type": "array"}
                    }
                })),
            },
            ToolDefinition {
                name: format!("{}.compression", self.name),
                description: "Bounded context compression via JEV".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {"type": "string"}
                    }
                })),
            },
        ]
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        match tool_name {
            name if name.ends_with(".approval") => self.handle_approval(args).await,
            name if name.ends_with(".mcp") => self.handle_mcp(args).await,
            name if name.ends_with(".risk") => self.handle_risk(args).await,
            name if name.ends_with(".priority") => self.handle_priority(args).await,
            name if name.ends_with(".retry") => self.handle_retry(args).await,
            name if name.ends_with(".resource") => self.handle_resource(args).await,
            name if name.ends_with(".compression") => self.handle_compression(args).await,
            _ => Err(crabjar_plugin::PluginError::Execution {
                name: self.name.clone(),
                reason: format!("Unknown tool: {}", tool_name),
            }),
        }
    }

    async fn health_check(&self) -> crabjar_plugin::HealthStatus {
        // Check if OpenRouter API key is configured.
        match self.get_api_key() {
            Ok(_) => crabjar_plugin::HealthStatus::Healthy,
            Err(e) => crabjar_plugin::HealthStatus::Unhealthy(e.to_string()),
        }
    }
}
