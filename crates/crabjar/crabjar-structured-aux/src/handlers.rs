//! Handler methods for each auxiliary task type.

use crate::plugin::StructuredAuxPlugin;
use crabjar_plugin::ToolResult;

/// Escape a string for JSON embedding.
fn escaped_json(s: &str) -> String {
    serde_json::to_string(&s).unwrap_or_default()
}

impl StructuredAuxPlugin {
    /// Handle approval task using structured criteria.
    pub async fn handle_approval(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let policy = args.get("policy").and_then(|v| v.as_str()).unwrap_or("");

        // Load structured criteria for approval decisions.
        let criteria = self.load_criteria("approval-decision")?;

        // Build context for the decision.
        let context = serde_json::json!({
            "request": request,
            "policy": policy
        });

        // Build structured prompt from criteria and context.
        let prompt = self.build_structured_prompt(&criteria, context);

        // Make the decision call with the structured prompt.
        let result = self
            .make_decision_call(serde_json::json!({}), vec![], Some(&prompt))
            .await?;
        let verdict = result
            .get("answer")
            .and_then(|v| v.as_str())
            .unwrap_or("needs_review");

        // Clean up the response - strip brackets if present.
        let verdict = verdict
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        Ok(ToolResult {
            stdout: format!("{{\"verdict\":\"{}\"}}", verdict),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle MCP tool selection task.
    pub async fn handle_mcp(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let tools_value = args.get("tools");

        // Parse available tools into options.
        let mut questions = vec![];
        if let Some(tools) = tools_value
            && let Some(tool_list) = tools.as_array()
        {
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

        let state = serde_json::json!({
            "request": request,
            "available_tools": args.get("tools")
        });

        let result = self.make_decision_call(state, questions, None).await?;

        // Extract the tool selection from the decision response.
        let tool = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"tool\":\"{}\"}}", tool),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle risk assessment task using structured criteria.
    pub async fn handle_risk(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");

        // Load structured criteria for risk assessment.
        let criteria = self.load_criteria("risk-assessment")?;

        // Build context for the decision.
        let decision_context = serde_json::json!({
            "action": action,
            "context": context
        });

        // Build structured prompt from criteria and context.
        let prompt = self.build_structured_prompt(&criteria, decision_context);

        // Make the decision call with the structured prompt.
        let result = self
            .make_decision_call(serde_json::json!({}), vec![], Some(&prompt))
            .await?;
        let risk = result
            .get("answer")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        // Clean up the response - strip brackets if present.
        let risk = risk
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        Ok(ToolResult {
            stdout: format!("{{\"risk_level\":\"{}\"}}", risk),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle priority ranking task.
    pub async fn handle_priority(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let tasks_value = args.get("tasks");
        let mut questions = vec![];

        if let Some(tasks) = tasks_value
            && let Some(task_list) = tasks.as_array()
        {
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

        let state = serde_json::json!({
            "tasks": tasks_value,
            "instruction": "Select the highest priority task"
        });

        let result = self.make_decision_call(state, questions, None).await?;
        let priority = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"highest_priority\":\"{}\"}}", priority),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle retry decision task using structured criteria.
    pub async fn handle_retry(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let error = args.get("error").and_then(|v| v.as_str()).unwrap_or("");
        let attempts = args.get("attempts").and_then(|v| v.as_i64()).unwrap_or(0);

        // Load structured criteria for retry decisions.
        let criteria = self.load_criteria("retry-decision")?;

        // Build context for the decision.
        let decision_context = serde_json::json!({
            "error": error,
            "attempts_made": attempts
        });

        // Build structured prompt from criteria and context.
        let prompt = self.build_structured_prompt(&criteria, decision_context);

        // Make the decision call with the structured prompt.
        let result = self
            .make_decision_call(serde_json::json!({}), vec![], Some(&prompt))
            .await?;
        let decision = result
            .get("answer")
            .and_then(|v| v.as_str())
            .unwrap_or("give_up");

        // Clean up the response - strip brackets if present.
        let decision = decision
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        Ok(ToolResult {
            stdout: format!("{{\"decision\":\"{}\"}}", decision),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle resource allocation task.
    pub async fn handle_resource(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
        let request = args.get("request").and_then(|v| v.as_str()).unwrap_or("");
        let resources_value = args.get("resources");

        let mut questions = vec![];
        if let Some(resources) = resources_value
            && let Some(resource_list) = resources.as_array()
        {
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

        let state = serde_json::json!({
            "request": request,
            "available_resources": resources_value
        });

        let result = self.make_decision_call(state, questions, None).await?;
        let resource = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"resource\":\"{}\"}}", resource),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }

    /// Handle compression task.
    pub async fn handle_compression(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult, crabjar_plugin::PluginError> {
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

        let result = self.make_decision_call(state, questions, None).await?;

        // Extract the compressed text from the answer field.
        let compressed = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult {
            stdout: format!("{{\"compressed\":{}}}", escaped_json(compressed)),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: None,
        })
    }
}
