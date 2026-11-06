//! StructuredAuxPlugin — bounded JEV decision-making via OpenRouter.

use crabjar_plugin::{Plugin, PluginError};
use reqwest::Client;
use std::env;
use std::fs;

/// OpenRouter JEV endpoint base URL.
const OPENROUTER_JEV_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// The structured-aux provider name used in crabjar's auxiliary task routing.
const PROVIDER_NAME: &str = "structured-aux";

/// Default JEV decision model from OpenRouter.
const DEFAULT_JEV_MODEL: &str = "~typesafe/jev-latest";

/// Structured auxiliary decision plugin.
pub struct StructuredAuxPlugin {
    pub(crate) name: String,
    pub(crate) version: String,
    client: Client,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    criteria_path: String,
}

impl StructuredAuxPlugin {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_config(None, None, api_key)
    }

    /// Create with explicit configuration.
    pub fn with_config(
        base_url: Option<String>,
        model: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        // Determine criteria path from environment, then relative paths.
        let criteria_path = env::var("STRUCTURED_AUX_CRITERIA_PATH")
            .or_else(|_| {
                env::current_dir().map(|p| {
                    p.join("crabjar-structured-aux/specs/criteria")
                        .to_string_lossy()
                        .to_string()
                })
            })
            .expect("STRUCTURED_AUX_CRITERIA_PATH must be set or run from repo root");

        Self {
            name: PROVIDER_NAME.to_string(),
            version: "0.1.0".to_string(),
            client: Client::new(),
            api_key,
            base_url,
            model,
            criteria_path,
        }
    }

    /// Load a criteria file by decision type name.
    pub(crate) fn load_criteria(
        &self,
        decision_type: &str,
    ) -> Result<serde_json::Value, crabjar_plugin::PluginError> {
        let path = format!("{}/{}.json", self.criteria_path, decision_type);
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).map_err(|e| PluginError::Execution {
                name: self.name.clone(),
                reason: format!("Failed to parse criteria {}: {}", path, e),
            }),
            Err(e) => Err(PluginError::Execution {
                name: self.name.clone(),
                reason: format!("Failed to read criteria file {}: {}", path, e),
            }),
        }
    }

    /// Build a structured prompt from criteria and context.
    pub(crate) fn build_structured_prompt(
        &self,
        criteria: &serde_json::Value,
        context: serde_json::Value,
    ) -> String {
        let mut prompt = String::new();

        // Add decision description if available.
        if let Some(description) = criteria.get("description").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("Decision: {}\n\n", description));
        }

        // Add evaluation factors as guidance.
        if let Some(factors) = criteria
            .get("evaluation_factors")
            .and_then(|v| v.as_array())
        {
            prompt.push_str("Consider these factors:\n");
            for (i, factor) in factors.iter().enumerate() {
                if let Some(factor_str) = factor.as_str() {
                    prompt.push_str(&format!("{}. {}\n", i + 1, factor_str));
                }
            }
            prompt.push('\n');
        }

        // Add the context/state.
        prompt.push_str("Context:\n");
        for (key, value) in context.as_object().unwrap_or(&serde_json::Map::new()) {
            if let Some(val_str) = value.as_str() {
                prompt.push_str(&format!("  {}: {}\n", key, val_str));
            } else {
                prompt.push_str(&format!("  {}: {}\n", key, value));
            }
        }
        prompt.push('\n');

        // Add options.
        if let Some(options) = criteria.get("options").and_then(|v| v.as_array()) {
            prompt.push_str("Available options:\n");
            for option in options.iter() {
                if let (Some(id), Some(desc)) = (
                    option.get("id").and_then(|v| v.as_str()),
                    option.get("description").and_then(|v| v.as_str()),
                ) {
                    prompt.push_str(&format!("[{}] {}\n", id, desc));
                }
            }
        }

        // Final instruction.
        prompt.push_str("\nSelect the most appropriate option by its ID.");

        prompt
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
    pub(crate) fn get_api_key(&self) -> Result<String, crabjar_plugin::PluginError> {
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
    pub async fn make_decision_call(
        &self,
        state: serde_json::Value,
        questions: Vec<serde_json::Value>,
        custom_prompt: Option<&str>,
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
        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": "You are a decision-making assistant. Given the state and available options, choose the most appropriate response. Return only the option name exactly as shown, nothing else."
        })];

        // Use custom prompt if provided, otherwise build from state and questions.
        let user_content = match custom_prompt {
            Some(prompt) => prompt.to_string(),
            None => {
                let mut content = format!("State: {}\n", state);
                for q in &questions {
                    if let Some(name) = q.get("name").and_then(|v| v.as_str()) {
                        content.push_str(&format!("Question: {}\n", name));
                        if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                            for opt in options.iter() {
                                if let Some(opt_str) = opt.as_str() {
                                    content.push_str(&format!("  {}\n", opt_str));
                                }
                            }
                        }
                    }
                }
                content
            }
        };

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
        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(PluginError::Serialization)?;

        // Debug: log raw response for troubleshooting.
        println!("DEBUG: Raw response from {}:", base_url);
        println!("DEBUG: {}", body);

        // Extract content from choices[0].message.content or reasoning_content.
        if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array())
            && !choices.is_empty()
            && let Some(message) = choices[0].get("message")
        {
            // Try content first, fall back to reasoning_content for reasoning models.
            let content = message
                .get("content")
                .and_then(|v| v.as_str())
                .or_else(|| message.get("reasoning_content").and_then(|v| v.as_str()));

            if let Some(text) = content {
                // Return the content as a JSON object with the answer.
                return Ok(serde_json::json!({ "answer": text }));
            }
        }

        Err(PluginError::Execution {
            name: self.name.clone(),
            reason: "No response content in choices".to_string(),
        })
    }
}
