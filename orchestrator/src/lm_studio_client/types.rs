//! Types for the LM Studio client: endpoints, config, and unified message types.

use serde::{Deserialize, Serialize};
use std::env;

// ---------------------------------------------------------------------------
// Endpoint selection
// ---------------------------------------------------------------------------

/// Which LM Studio endpoint to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LmStudioEndpoint {
    /// Native `/api/v1/chat` endpoint — supports stateful chat.
    Native,
    /// OpenAI-compatible `/v1/chat/completions` endpoint.
    Openai,
    /// Anthropic-compatible `/v1/messages` endpoint.
    Anthropic,
    /// Mistral.rs serve OpenAI-compatible endpoint.
    MistralRsServe,
}

impl LmStudioEndpoint {
    /// Parses from environment variable `LM_STUDIO_ENDPOINT`.
    /// Defaults to `Openai` if not set or unrecognized.
    pub fn from_env() -> Self {
        match env::var("LM_STUDIO_ENDPOINT").ok().as_deref() {
            Some("native") => Self::Native,
            Some("openai") => Self::Openai,
            Some("anthropic") => Self::Anthropic,
            Some("mistralrs") => Self::MistralRsServe,
            _ => Self::Openai,
        }
    }

    /// Returns the URL path for this endpoint.
    pub fn path(&self) -> &'static str {
        match self {
            Self::Native => "/api/v1/chat",
            Self::Openai => "/v1/chat/completions",
            Self::Anthropic => "/v1/messages",
            Self::MistralRsServe => "/v1/chat/completions",
        }
    }

    /// Returns the display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Openai => "openai-compat",
            Self::Anthropic => "anthropic-compat",
            Self::MistralRsServe => "mistralrs-serve",
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for connecting to LM Studio.
#[derive(Debug, Clone)]
pub struct LmStudioConfig {
    /// Base URL of the LM Studio server (e.g. `http://127.0.0.1:1234`).
    pub base_url: String,
    /// Base URL for mistral.rs serve (overrides `base_url` when endpoint is `MistralRsServe`).
    pub serve_base_url: Option<String>,
    /// Which endpoint to use.
    pub endpoint: LmStudioEndpoint,
    /// API token for authentication (optional).
    pub api_token: Option<String>,
    /// Default model to use if not specified in a request.
    pub default_model: String,
    /// Default context length in tokens.
    pub default_context_length: Option<i64>,
    /// Default temperature.
    pub default_temperature: Option<f64>,
    /// Default max output tokens.
    pub default_max_output_tokens: Option<i64>,
}

impl LmStudioConfig {
    /// Loads configuration from environment variables.
    pub fn from_env() -> Self {
        let base_url =
            env::var("LM_STUDIO_URL").unwrap_or_else(|_| "http://127.0.0.1:1234".to_string());

        let serve_base_url = env::var("MISTRALRS_SERVE_URL").ok();

        let endpoint = LmStudioEndpoint::from_env();

        let api_token = env::var("LM_API_TOKEN").ok();

        let default_model =
            env::var("LM_STUDIO_MODEL").unwrap_or_else(|_| "local-model".to_string());

        let default_context_length = env::var("LM_STUDIO_CONTEXT_LENGTH")
            .ok()
            .and_then(|v| v.parse::<i64>().ok());

        let default_temperature = env::var("LM_STUDIO_TEMPERATURE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok());

        let default_max_output_tokens = env::var("LM_STUDIO_MAX_OUTPUT_TOKENS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok());

        Self {
            base_url,
            serve_base_url,
            endpoint,
            api_token,
            default_model,
            default_context_length,
            default_temperature,
            default_max_output_tokens,
        }
    }

    /// Returns the full URL for the configured endpoint.
    ///
    /// Uses `serve_base_url` when the endpoint is `MistralRsServe`,
    /// otherwise falls back to `base_url`.
    pub fn endpoint_url(&self) -> String {
        let base = match self.endpoint {
            LmStudioEndpoint::MistralRsServe => {
                self.serve_base_url.as_deref().unwrap_or(&self.base_url)
            }
            _ => &self.base_url,
        };
        format!("{}{}", base, self.endpoint.path())
    }
}

// ---------------------------------------------------------------------------
// Unified request/response types
// ---------------------------------------------------------------------------

/// A unified chat message that works across all endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMessage {
    /// The role of the message.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
}

/// Message role in a chat conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A unified chat request that works across all endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedChatRequest {
    /// Model to use (overrides config default if set).
    pub model: String,
    /// The message to send.
    pub input: UnifiedMessage,
    /// System prompt (optional).
    pub system_prompt: Option<String>,
    /// Temperature (overrides config default if set).
    pub temperature: Option<f64>,
    /// Top P (overrides config default if set).
    pub top_p: Option<f64>,
    /// Max output tokens (overrides config default if set).
    pub max_output_tokens: Option<i64>,
    /// Context length in tokens (overrides config default if set).
    pub context_length: Option<i64>,
    /// Previous response ID for stateful continuation (native endpoint).
    pub previous_response_id: Option<String>,
    /// Whether to store the chat (native endpoint).
    pub store: Option<bool>,
    /// Reasoning setting.
    pub reasoning: Option<ReasoningLevel>,
}

impl UnifiedChatRequest {
    /// Builds a request from config defaults, overriding with explicit values.
    pub fn from_config(
        config: &LmStudioConfig,
        user_input: String,
        previous_response_id: Option<String>,
    ) -> Self {
        Self {
            model: config.default_model.clone(),
            input: UnifiedMessage {
                role: MessageRole::User,
                content: user_input,
            },
            system_prompt: None,
            temperature: config.default_temperature,
            top_p: None,
            max_output_tokens: config.default_max_output_tokens,
            context_length: config.default_context_length,
            previous_response_id,
            store: Some(true),
            reasoning: None,
        }
    }
}

/// Reasoning level for the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLevel {
    Off,
    Low,
    Medium,
    High,
    On,
}

/// A unified chat response that works across all endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedChatResponse {
    /// The model instance that generated the response.
    pub model_instance_id: String,
    /// The output from the model.
    pub output: Vec<UnifiedOutputItem>,
    /// Token usage statistics.
    pub stats: Option<UnifiedStats>,
    /// Response ID for stateful continuation (native endpoint).
    pub response_id: Option<String>,
}

/// An output item from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UnifiedOutputItem {
    /// A text message from the model.
    Message { content: String },
    /// A tool call made by the model.
    ToolCall {
        tool: String,
        arguments: serde_json::Value,
        output: Option<String>,
        provider_info: Option<ToolProviderInfo>,
    },
    /// Reasoning content from the model.
    Reasoning { content: String },
    /// An invalid tool call.
    InvalidToolCall {
        reason: String,
        metadata: Option<serde_json::Value>,
        tool_name: Option<String>,
        provider_info: Option<ToolProviderInfo>,
    },
}

/// Information about a tool provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProviderInfo {
    /// The provider type.
    pub provider_type: String,
    /// Plugin ID (for plugin type).
    pub plugin_id: Option<String>,
    /// Server label (for ephemeral MCP type).
    pub server_label: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedStats {
    /// Input tokens consumed.
    pub input_tokens: i64,
    /// Total output tokens generated.
    pub total_output_tokens: i64,
    /// Reasoning output tokens.
    pub reasoning_output_tokens: Option<i64>,
    /// Tokens per second.
    pub tokens_per_second: Option<f64>,
    /// Time to first token in seconds.
    pub time_to_first_token_seconds: Option<f64>,
    /// Model load time in seconds (if applicable).
    pub model_load_time_seconds: Option<f64>,
}
