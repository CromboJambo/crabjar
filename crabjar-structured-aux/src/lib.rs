//! # crabjar-structured-aux
//!
//! Structured auxiliary decision calls via OpenRouter's JEV endpoint.
//! Routes selected Hermes auxiliary tasks through bounded Jev decision calls
//! instead of free-form chat prompts for improved reliability and cost efficiency.
//!
//! This plugin replaces the `hermes-structured-aux-models` Python plugin with a native
//! Rust implementation that integrates directly into crabjar's plugin system.

use async_trait::async_trait;
use crabjar_plugin::{Plugin, PluginError, ToolDefinition, ToolResult};
use reqwest::Client;
use serde_json;
use std::env;

/// OpenRouter JEV endpoint base URL.
const OPENROUTER_JEV_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// The structured-aux provider name used in crabjar's auxiliary task routing.
const PROVIDER_NAME: &str = "structured-aux";

/// Task type identifiers for routing.
// mod tasks {
//     pub const APPROVAL: &str = "approval";
//     pub const MCP: &str = "mcp";
//     pub const COMPRESSION: &str = "compression";
// }

/// Default JEV decision model from OpenRouter.
const DEFAULT_JEV_MODEL: &str = "~typesafe/jev-latest";

/// Structured auxiliary decision plugin.
pub struct StructuredAuxPlugin {
    name: String,
    version: String,
    client: Client,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
}

impl StructuredAuxPlugin {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_config(None, None, api_key)
    }

    /// Create with explicit configuration.
    pub fn with_config(base_url: Option<String>, model: Option<String>, api_key: Option<String>) -> Self {
        Self {
            name: PROVIDER_NAME.to_string(),
            version: "0.1.0".to_string(),
            client: Client::new(),
            api_key,
            base_url,
            model,
        }
    }

    /// Get the configured base URL, falling back to OpenRouter.
    fn get_base_url(&self) -> String {
        match &self.base_url {
            Some(url) => url.clone(),
            None => OPENROUTER_JEV_BASE_URL.to_string(),
        }
    }

    /// Get the configured model name, falling back to default JEV model.
    fn get_model(&self) -> String {
        match &self.model {
            Some(model) => model.clone(),
            None => DEFAULT_JEV_MODEL.to_string(),
        }
    }

    /// Get the OpenRouter API key from environment or config.
    fn get_api_key(&self) -> Result<String, PluginError> {
        if let Some(key) = &self.api_key {
            return Ok(key.clone());
        }
        match env::var("OPENROUTER_API_KEY") {
            Ok(key) => Ok(key),
            Err(_) => Err(PluginError::Execution {
                name: self.name.clone(),
                reason: "OPENROUTER_API_KEY not set".to_string(),
            }),
        }
    }

    /// Make a bounded JEV decision call.
    async fn make_decision_call(
        &self,
        state: serde_json::Value,
        questions: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, PluginError> {
        let base_url = self.get_base_url();
        let model = self.get_model();

        // For local llama-server instances, API key is optional.
        let api_key = match &self.api_key {
            Some(key) => key.clone(),
            None => {
                if base_url == OPENROUTER_JEV_BASE_URL {
                    return Err(PluginError::Execution {
                        name: self.name.clone(),
                        reason: "OPENROUTER_API_KEY not set".to_string(),
                    });
                }
                String::new()
            }
        };

        // Construct the request based on whether we're using OpenRouter or a local llama-server.
        let url = format!("{}/v1/chat/completions", base_url);

        // Build a system prompt that instructs bounded decision-making.
        let mut messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a decision-making assistant. Given the state and available options, choose the most appropriate response. Return only the option name, nothing else."
            })
        ];

        // Build user message with state and questions.
        let mut user_content = format!("State: {}\n", state.to_string());
        for q in &questions {
            if let Some(name) = q.get("name").and_then(|v| v.as_str()) {
                user_content.push_str(&format!("Question: {}\n", name));
                if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                    for opt in options.iter() {
                        if let Some(opt_str) = opt.as_str() {
                            user_content.push_str(&format!("  {}\n", opt_str));
                        }
                    }
                }
            }
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_content
        }));

        // Use higher max_tokens for compression tasks (reasoning models need more space).
        let max_tokens = if state.get("text_to_compress").is_some() {
            200
        } else {
            50
        };

        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": 0.1,
            "max_tokens": max_tokens
        });

        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(request.to_string());

        // Add authorization header if API key is available.
        if !api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await.map_err(|e| PluginError::Execution {
            name: self.name.clone(),
            reason: format!("HTTP request failed: {}", e),
        })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| PluginError::Execution {
            name: self.name.clone(),
            reason: format!("Failed to read response: {}", e),
        })?;

        if !status.is_success() {
            return Err(PluginError::Execution {
                name: self.name.clone(),
                reason: format!("API error ({}): {}", status, body),
            });
        }

        // Parse the chat completion response.
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| PluginError::Serialization(e.into()))?;

        // Debug: log raw response for troubleshooting.
        println!("DEBUG: Raw response from {}:", base_url);
        println!("DEBUG: {}", body);

        // Extract content from choices[0].message.content or reasoning_content.
        if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
            if !choices.is_empty() {
                if let Some(message) = choices[0].get("message") {
                    // Try content first, fall back to reasoning_content for reasoning models.
                    let content = message.get("content").and_then(|v| v.as_str())
                        .or_else(|| message.get("reasoning_content").and_then(|v| v.as_str()));
                    
                    if let Some(text) = content {
                        // Return the content as a JSON object with the answer.
                        return Ok(serde_json::json!({ "answer": text }));
                    }
                }
            }
        }

        Err(PluginError::Execution {
            name: self.name.clone(),
            reason: "No response content in choices".to_string(),
        })
    }

    /// Handle approval task.
    async fn handle_approval(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let policy = args.get("policy").and_then(|v| v.as_str()).unwrap_or("");

        // Define bounded approval options.
        let questions = vec![serde_json::json!({
            "name": "verdict",
            "options": ["approve", "deny", "needs_review"]
        })];

        let state = serde_json::json!({
            "request": request,
            "guardian_policy": policy
        });

        let result = self.make_decision_call(state, questions).await?;

        // Extract the verdict from the decision response.
        let verdict = result.get("answer").and_then(|v| v.as_str()).unwrap_or("needs_review");

        Ok(ToolResult {
            stdout: format!("{{\"verdict\":\"{}\"}}", verdict),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle MCP tool selection task.
    async fn handle_mcp(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let tools_value = args.get("tools");

        // Parse available tools into options.
        let mut questions = vec![];
        if let Some(tools) = tools_value {
            if let Some(tool_list) = tools.as_array() {
                let tool_names: Vec<String> = tool_list
                    .iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect();

                if !tool_names.is_empty() {
                    questions.push(serde_json::json!({
                        "name": "tool",
                        "options": tool_names
                    }));
                }
            }
        }

        let state = serde_json::json!({
            "request": request,
            "available_tools": args.get("tools")
        });

        let result = self.make_decision_call(state, questions).await?;

        // Extract the tool selection from the decision response.
        let tool = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"tool\":\"{}\"}}", tool),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle risk assessment task.
    async fn handle_risk(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");

        let questions = vec![serde_json::json!({
            "name": "risk_level",
            "options": ["low", "medium", "high", "critical"]
        })];

        let state = serde_json::json!({
            "action": action,
            "context": context
        });

        let result = self.make_decision_call(state, questions).await?;
        let risk = result.get("answer").and_then(|v| v.as_str()).unwrap_or("medium");

        Ok(ToolResult {
            stdout: format!("{{\"risk_level\":\"{}\"}}", risk),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle priority ranking task.
    async fn handle_priority(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let tasks_value = args.get("tasks");
        let mut questions = vec![];

        if let Some(tasks) = tasks_value {
            if let Some(task_list) = tasks.as_array() {
                let task_names: Vec<String> = task_list
                    .iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect();

                if !task_names.is_empty() {
                    questions.push(serde_json::json!({
                        "name": "highest_priority",
                        "options": task_names
                    }));
                }
            }
        }

        let state = serde_json::json!({
            "tasks": tasks_value,
            "instruction": "Select the highest priority task"
        });

        let result = self.make_decision_call(state, questions).await?;
        let priority = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"highest_priority\":\"{}\"}}", priority),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle retry decision task.
    async fn handle_retry(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let error = args.get("error").and_then(|v| v.as_str()).unwrap_or("");
        let attempts = args.get("attempts").and_then(|v| v.as_i64()).unwrap_or(0);

        let questions = vec![serde_json::json!({
            "name": "should_retry",
            "options": ["retry", "give_up"]
        })];

        let state = serde_json::json!({
            "error": error,
            "attempts_made": attempts
        });

        let result = self.make_decision_call(state, questions).await?;
        let decision = result.get("answer").and_then(|v| v.as_str()).unwrap_or("give_up");

        Ok(ToolResult {
            stdout: format!("{{\"decision\":\"{}\"}}", decision),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle resource allocation task.
    async fn handle_resource(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let resources_value = args.get("resources");

        let mut questions = vec![];
        if let Some(resources) = resources_value {
            if let Some(resource_list) = resources.as_array() {
                let resource_names: Vec<String> = resource_list
                    .iter()
                    .filter_map(|r| r.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect();

                if !resource_names.is_empty() {
                    questions.push(serde_json::json!({
                        "name": "resource",
                        "options": resource_names
                    }));
                }
            }
        }

        let state = serde_json::json!({
            "request": request,
            "available_resources": resources_value
        });

        let result = self.make_decision_call(state, questions).await?;
        let resource = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"resource\":\"{}\"}}", resource),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle compression task.
    async fn handle_compression(&self, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");

        // Compression uses a different approach — summarize the text into a compact form.
        let questions = vec![serde_json::json!({
            "name": "compressed",
            "options": []
        })];

        let state = serde_json::json!({
            "text_to_compress": text,
            "instruction": "Compress this context into a compact summary preserving key facts."
        });

        let result = self.make_decision_call(state, questions).await?;

        // Extract the compressed text from the answer field.
        let compressed = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"compressed\":\"{}\"}}", escaped_json(compressed)),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }
}

/// Escape a string for JSON embedding.
fn escaped_json(s: &str) -> String {
    serde_json::to_string(&s).unwrap_or_default()
}

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
                description: "Bounded approval decision via JEV (approve/deny/needs_review)".to_string(),
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
                description: "Bounded risk assessment via JEV (low/medium/high/critical)".to_string(),
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

    async fn execute_tool(&self, tool_name: &str, args: serde_json::Value) -> Result<ToolResult, PluginError> {
        match tool_name {
            name if name.ends_with(".approval") => self.handle_approval(args).await,
            name if name.ends_with(".mcp") => self.handle_mcp(args).await,
            name if name.ends_with(".risk") => self.handle_risk(args).await,
            name if name.ends_with(".priority") => self.handle_priority(args).await,
            name if name.ends_with(".retry") => self.handle_retry(args).await,
            name if name.ends_with(".resource") => self.handle_resource(args).await,
            name if name.ends_with(".compression") => self.handle_compression(args).await,
            _ => Err(PluginError::Execution {
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

/// Factory function to create the structured aux plugin.
pub fn create_structured_aux_plugin(api_key: Option<String>) -> StructuredAuxPlugin {
    StructuredAuxPlugin::new(api_key)
}