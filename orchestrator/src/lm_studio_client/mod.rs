/// lm_studio_client: Unified client for LM Studio's multiple API endpoints.
///
/// Supports three endpoints with a toggle:
/// - Native `/api/v1/chat` — stateful chat via `previous_response_id`
/// - OpenAI-compatible `/v1/chat/completions` — full message history
/// - Anthropic-compatible `/v1/messages` — full message history
///
/// The client abstracts endpoint differences so the orchestrator doesn't
/// need to know which endpoint it's talking to.
///
/// Session state is managed via `SessionStore` — for the native endpoint
/// this tracks `response_id` for continuation; for OpenAI/Anthropic it
/// tracks the full message history.

use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;
use tracing::{debug, error, info, warn};

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
        let base_url = env::var("LM_STUDIO_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234".to_string());

        let serve_base_url = env::var("MISTRALRS_SERVE_URL").ok();

        let endpoint = LmStudioEndpoint::from_env();

        let api_token = env::var("LM_API_TOKEN").ok();

        let default_model = env::var("LM_STUDIO_MODEL")
            .unwrap_or_else(|_| "local-model".to_string());

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

// ---------------------------------------------------------------------------
// Session state management
// ---------------------------------------------------------------------------

/// Tracks session state for stateful chat continuation.
///
/// For the native endpoint, this stores the `response_id` so the next
/// request can use `previous_response_id` to continue the conversation.
///
/// For OpenAI/Anthropic endpoints, this stores the full message history
/// so it can be re-sent on each turn.
#[derive(Debug, Clone)]
pub struct SessionState {
    /// The current response ID (native endpoint).
    pub response_id: Option<String>,
    /// The full message history (OpenAI/Anthropic endpoints).
    pub message_history: Vec<UnifiedMessage>,
}

impl SessionState {
    /// Creates a new empty session.
    pub fn new() -> Self {
        Self {
            response_id: None,
            message_history: Vec::new(),
        }
    }

    /// Initializes a session with a system message.
    pub fn with_system_prompt(system_prompt: String) -> Self {
        Self {
            response_id: None,
            message_history: vec![UnifiedMessage {
                role: MessageRole::System,
                content: system_prompt,
            }],
        }
    }

    /// Updates the session state with a response from the model.
    pub fn update_with_response(&mut self, response: &UnifiedChatResponse) {
        // Store the response ID for stateful continuation.
        if let Some(ref rid) = response.response_id {
            self.response_id = Some(rid.clone());
        }

        // Collect assistant messages from the response output.
        for item in &response.output {
            match item {
                UnifiedOutputItem::Message { content } => {
                    self.message_history.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: content.clone(),
                    });
                }
                UnifiedOutputItem::ToolCall { tool, output, .. } => {
                    if let Some(result) = output {
                        self.message_history.push(UnifiedMessage {
                            role: MessageRole::Assistant,
                            content: format!("Tool '{}' executed: {}", tool, result),
                        });
                    }
                }
                UnifiedOutputItem::Reasoning { content } => {
                    self.message_history.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: format!("[reasoning] {}", content),
                    });
                }
                _ => {}
            }
        }
    }

    /// Adds a user message to the session.
    pub fn add_user_message(&mut self, content: String) {
        self.message_history.push(UnifiedMessage {
            role: MessageRole::User,
            content,
        });
    }

    /// Returns whether this session has a response ID for stateful continuation.
    pub fn has_response_id(&self) -> bool {
        self.response_id.is_some()
    }
}

// ---------------------------------------------------------------------------
// SQLite-backed session store
// ---------------------------------------------------------------------------

/// SQLite-backed session store for persisting chat state across process restarts.
///
/// This complements LM Studio's native stateful chat by storing session
/// metadata and message history in SQLite. When the orchestrator restarts,
/// it can restore sessions from the database.
#[derive(Debug)]
pub struct SessionStore {
    /// SQLite database path.
    pub db_path: String,
    /// Connection to the database (opened lazily).
    conn: std::sync::Mutex<Option<rusqlite::Connection>>,
}

impl SessionStore {
    /// Creates a new session store with the given database path.
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            conn: std::sync::Mutex::new(None),
        }
    }

    /// Opens the database connection lazily.
    fn open_conn(&self) -> Result<std::sync::MutexGuard<'_, Option<rusqlite::Connection>>, SessionError> {
        let mut guard = self.conn.lock().map_err(|e| {
            SessionError::Database(format!("failed to lock connection: {e}"))
        })?;
        if guard.is_none() {
            let conn = rusqlite::Connection::open(&self.db_path)
                .map_err(|e| SessionError::Database(format!("failed to open DB: {e}")))?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE IF NOT EXISTS sessions (
                     id TEXT PRIMARY KEY,
                     system_prompt TEXT,
                     message_history TEXT,
                     response_id TEXT,
                     created_at INTEGER DEFAULT (unixepoch()),
                     updated_at INTEGER DEFAULT (unixepoch())
                 );
                 CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);",
            )
            .map_err(|e| SessionError::Database(format!("failed to init schema: {e}")))?;
            *guard = Some(conn);
        }
        Ok(guard)
    }

    /// Creates a new session and returns its ID.
    ///
    /// The session is initialized with a system prompt.
    pub fn create_session(&self, system_prompt: Option<String>) -> Result<String, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let system_prompt_json = serde_json::to_string(&system_prompt).unwrap_or_else(|_| "null".to_string());

        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, system_prompt, message_history, response_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                &session_id,
                system_prompt_json,
                "[]", // empty message history
                "",   // no response_id yet
            ],
        )
        .map_err(|e| SessionError::Database(format!("failed to create session: {e}")))?;

        debug!(
            "Created session {} with system prompt: {:?}",
            session_id, system_prompt
        );

        Ok(session_id)
    }

    /// Retrieves session state by ID.
    pub fn get_session(&self, session_id: &str) -> Result<SessionState, SessionError> {
        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        let mut stmt = conn.prepare(
            "SELECT system_prompt, message_history, response_id FROM sessions WHERE id = ?1",
        )
        .map_err(|e| SessionError::Database(format!("failed to prepare query: {e}")))?;

        let row = stmt.query_row(rusqlite::params![session_id], |row| {
            let system_prompt: String = row.get(0)?;
            let message_history: String = row.get(1)?;
            let response_id: String = row.get(2)?;
            Ok((system_prompt, message_history, response_id))
        })
        .ok(); // stub: return None for missing sessions

        let (system_prompt, message_history, response_id) = match row {
            Some(r) => (r.0, r.1, r.2),
            None => (String::new(), String::new(), String::new()),
        };

        let system_prompt: Option<String> = serde_json::from_str(&system_prompt).unwrap_or(None);
        let message_history: Vec<UnifiedMessage> = serde_json::from_str(&message_history).unwrap_or_default();
        let response_id = if response_id.is_empty() { None } else { Some(response_id) };

        let mut state = SessionState::new();
        state.response_id = response_id;
        for msg in message_history {
            state.message_history.push(msg);
        }

        debug!("Retrieved session {}", session_id);
        Ok(state)
    }

    /// Updates session state in the store.
    pub fn update_session(
        &self,
        session_id: &str,
        state: &SessionState,
    ) -> Result<(), SessionError> {
        let message_history_json = serde_json::to_string(&state.message_history)
            .unwrap_or_else(|_| "[]".to_string());
        let response_id = state.response_id.as_deref().unwrap_or("");

        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        conn.execute(
            "UPDATE sessions SET message_history = ?1, response_id = ?2, updated_at = unixepoch() WHERE id = ?3",
            rusqlite::params![message_history_json, response_id, session_id],
        )
        .map_err(|e| SessionError::Database(format!("failed to update session: {e}")))?;

        debug!("Updated session {}", session_id);
        Ok(())
    }

    /// Deletes a session. Idempotent — returns Ok even if the session doesn't exist.
    pub fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        let _rows = conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )
        .map_err(|e| SessionError::Database(format!("failed to delete session: {e}")))?;

        debug!("Deleted session {}", session_id);
        Ok(())
    }
}

/// Errors from session store operations.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Database(String),
}

// ---------------------------------------------------------------------------
// Endpoint implementations
// ---------------------------------------------------------------------------

/// Native `/api/v1/chat` endpoint implementation.
mod native {
    use super::*;
    use reqwest::Client;

    /// Converts a unified request to the native endpoint format.
    pub fn to_native_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut builder = serde_json::Map::new();

        builder.insert("model".to_string(), serde_json::Value::String(req.model.clone()));

        // Convert input to native format.
        let input_obj = serde_json::json!({
            "type": "message",
            "content": req.input.content
        });
        builder.insert("input".to_string(), serde_json::Value::Array(vec![input_obj]));

        if let Some(ref system_prompt) = req.system_prompt {
            builder.insert(
                "system_prompt".to_string(),
                serde_json::Value::String(system_prompt.clone()),
            );
        }

        if let Some(temp) = req.temperature {
            builder.insert("temperature".to_string(), serde_json::to_value(temp).unwrap_or(serde_json::Value::Null));
        }

        if let Some(top_p) = req.top_p {
            builder.insert("top_p".to_string(), serde_json::to_value(top_p).unwrap_or(serde_json::Value::Null));
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        if let Some(ctx_len) = req.context_length {
            builder.insert(
                "context_length".to_string(),
                serde_json::Value::Number(ctx_len.into()),
            );
        }

        if let Some(ref prev_id) = req.previous_response_id {
            builder.insert(
                "previous_response_id".to_string(),
                serde_json::Value::String(prev_id.clone()),
            );
        }

        if let Some(store) = req.store {
            builder.insert("store".to_string(), serde_json::Value::Bool(store));
        }

        if let Some(reasoning) = req.reasoning {
            let reasoning_str = match reasoning {
                ReasoningLevel::Off => "off",
                ReasoningLevel::Low => "low",
                ReasoningLevel::Medium => "medium",
                ReasoningLevel::High => "high",
                ReasoningLevel::On => "on",
            };
            builder.insert(
                "reasoning".to_string(),
                serde_json::Value::String(reasoning_str.to_string()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts a native response to the unified format.
    pub fn from_native_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, native_error::NativeError> {
        let model_instance_id = value
            .get("model_instance_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let output_items: Vec<UnifiedOutputItem> = value
            .get("output")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| parse_output_item(item))
                    .collect()
            })
            .unwrap_or_default();

        let stats = value
            .get("stats")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        let response_id = value
            .get("response_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(UnifiedChatResponse {
            model_instance_id,
            output: output_items,
            stats,
            response_id,
        })
    }

    /// Parses a single output item from a native response.
    fn parse_output_item(item: &serde_json::Value) -> Option<UnifiedOutputItem> {
        let item_type = item.get("type")?.as_str()?;

        match item_type {
            "message" => {
                let content = item.get("content")?.as_str()?.to_string();
                Some(UnifiedOutputItem::Message { content })
            }
            "tool_call" => {
                let tool = item.get("tool")?.as_str()?.to_string();
                let arguments = item.get("arguments")?.clone();
                let output = item.get("output")?.as_str().map(|s| s.to_string());
                let provider_info = item
                    .get("provider_info")
                    .and_then(|v| serde_json::from_value::<ToolProviderInfo>(v.clone()).ok());
                Some(UnifiedOutputItem::ToolCall {
                    tool,
                    arguments,
                    output,
                    provider_info,
                })
            }
            "reasoning" => {
                let content = item.get("content")?.as_str()?.to_string();
                Some(UnifiedOutputItem::Reasoning { content })
            }
            "invalid_tool_call" => {
                let reason = item.get("reason")?.as_str()?.to_string();
                let metadata = item.get("metadata")?.clone();
                let tool_name = item.get("tool_name")?.as_str().map(|s| s.to_string());
                let provider_info = item
                    .get("provider_info")
                    .and_then(|v| serde_json::from_value::<ToolProviderInfo>(v.clone()).ok());
                Some(UnifiedOutputItem::InvalidToolCall {
                    reason,
                    metadata: Some(metadata),
                    tool_name,
                    provider_info,
                })
            }
            _ => None,
        }
    }

    /// Native endpoint error types.
    pub mod native_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum NativeError {
            #[error("failed to parse native response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}

/// OpenAI-compatible `/v1/chat/completions` endpoint implementation.
mod openai {
    use super::*;
    use reqwest::Client;

    /// Converts a unified request to the OpenAI format.
    pub fn to_openai_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Add system message if present.
        if let Some(ref system_prompt) = req.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Add the input message.
        let role_str = match req.input.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        messages.push(serde_json::json!({
            "role": role_str,
            "content": req.input.content
        }));

        let mut builder = serde_json::Map::new();
        builder.insert("model".to_string(), serde_json::Value::String(req.model.clone()));
        builder.insert("messages".to_string(), serde_json::Value::Array(messages));

        if let Some(temp) = req.temperature {
            builder.insert("temperature".to_string(), serde_json::to_value(temp).unwrap_or(serde_json::Value::Null));
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts an OpenAI response to the unified format.
    pub fn from_openai_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, openai_error::OpenaiError> {
        let choices = value
            .get("choices")
            .and_then(|v| v.as_array())
            .ok_or(openai_error::OpenaiError::ParseError(
                "missing choices in response".to_string(),
            ))?;

        if choices.is_empty() {
            return Err(openai_error::OpenaiError::ParseError(
                "empty choices in response".to_string(),
            ));
        }

        let first_choice = &choices[0];
        let message = first_choice
            .get("message")
            .ok_or(openai_error::OpenaiError::ParseError(
                "missing message in choice".to_string(),
            ))?;

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls: Vec<UnifiedOutputItem> = message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| parse_tool_call(tc))
                    .collect()
            })
            .unwrap_or_default();

        // Combine message and tool calls into output items.
        let mut output_items = Vec::new();
        if !content.is_empty() {
            output_items.push(UnifiedOutputItem::Message { content });
        }
        output_items.extend(tool_calls);

        let stats = value
            .get("usage")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        Ok(UnifiedChatResponse {
            model_instance_id: value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
            output: output_items,
            stats,
            response_id: None,
        })
    }

    /// Parses a tool call from an OpenAI response.
    fn parse_tool_call(tc: &serde_json::Value) -> Option<UnifiedOutputItem> {
        let function = tc.get("function")?;
        let name = function.get("name")?.as_str()?.to_string();
        let arguments_str = function.get("arguments")?.as_str()?;
        let arguments = serde_json::from_str(arguments_str).ok()?;

        Some(UnifiedOutputItem::ToolCall {
            tool: name,
            arguments,
            output: None,
            provider_info: None,
        })
    }

    /// OpenAI endpoint error types.
    pub mod openai_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum OpenaiError {
            #[error("failed to parse OpenAI response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}

/// Anthropic-compatible `/v1/messages` endpoint implementation.
mod anthropic {
    use super::*;
    use reqwest::Client;

    /// Converts a unified request to the Anthropic format.
    pub fn to_anthropic_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Anthropic doesn't have a system message field at the top level.
        // System messages are prepended to the first user message.
        if let Some(ref system_prompt) = req.system_prompt {
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!("[System prompt: {}]", system_prompt)
            }));
        }

        // Add the input message.
        let role_str = match req.input.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        messages.push(serde_json::json!({
            "role": role_str,
            "content": req.input.content
        }));

        let mut builder = serde_json::Map::new();
        builder.insert("model".to_string(), serde_json::Value::String(req.model.clone()));
        builder.insert("messages".to_string(), serde_json::Value::Array(messages));

        if let Some(temp) = req.temperature {
            builder.insert("temperature".to_string(), serde_json::to_value(temp).unwrap_or(serde_json::Value::Null));
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts an Anthropic response to the unified format.
    pub fn from_anthropic_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, anthropic_error::AnthropicError> {
        let content_blocks = value
            .get("content")
            .and_then(|v| v.as_array())
            .ok_or(anthropic_error::AnthropicError::ParseError(
                "missing content in response".to_string(),
            ))?;

        let mut output_items = Vec::new();
        for block in content_blocks {
            let block_type = block.get("type").ok_or(anthropic_error::AnthropicError::ParseError("missing type".to_string()))?.as_str().ok_or(anthropic_error::AnthropicError::ParseError("type not a string".to_string()))?;

            match block_type {
                "text" => {
                    let content = block.get("text").ok_or(anthropic_error::AnthropicError::ParseError("missing text".to_string()))?.as_str().ok_or(anthropic_error::AnthropicError::ParseError("text not a string".to_string()))?.to_string();
                    output_items.push(UnifiedOutputItem::Message { content });
                }
                "tool_use" => {
                    let name = block.get("name").ok_or(anthropic_error::AnthropicError::ParseError("missing name".to_string()))?.as_str().ok_or(anthropic_error::AnthropicError::ParseError("name not a string".to_string()))?.to_string();
                    let input = block.get("input").ok_or(anthropic_error::AnthropicError::ParseError("missing input".to_string()))?.clone();
                    output_items.push(UnifiedOutputItem::ToolCall {
                        tool: name,
                        arguments: input,
                        output: None,
                        provider_info: None,
                    });
                }
                _ => {}
            }
        }

        let stats = value
            .get("usage")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        Ok(UnifiedChatResponse {
            model_instance_id: value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
            output: output_items,
            stats,
            response_id: None,
        })
    }

    /// Anthropic endpoint error types.
    pub mod anthropic_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum AnthropicError {
            #[error("failed to parse Anthropic response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}

// ---------------------------------------------------------------------------
// Unified client
// ---------------------------------------------------------------------------

/// The unified LM Studio client that abstracts endpoint differences.
///
/// It routes requests to the configured endpoint and converts responses
/// to the unified format so the orchestrator doesn't need to know which
/// endpoint it's talking to.
#[derive(Debug)]
pub struct LmStudioClient {
    /// Configuration.
    config: LmStudioConfig,
    /// HTTP client.
    http_client: reqwest::Client,
    /// Session state for stateful continuation.
    session: SessionState,
    /// Session store for persistence.
    session_store: Option<SessionStore>,
    /// Current session ID.
    current_session_id: Option<String>,
}

impl LmStudioClient {
    /// Creates a new client from environment configuration.
    pub fn from_env() -> Self {
        let config = LmStudioConfig::from_env();
        let http_client = reqwest::Client::new();
        let session = SessionState::new();

        Self {
            config,
            http_client,
            session,
            session_store: None,
            current_session_id: None,
        }
    }

    /// Creates a new client with explicit configuration.
    pub fn new(config: LmStudioConfig) -> Self {
        let http_client = reqwest::Client::new();
        let session = SessionState::new();

        Self {
            config,
            http_client,
            session,
            session_store: None,
            current_session_id: None,
        }
    }

    /// Sets the session store for persistence.
    pub fn with_session_store(mut self, store: SessionStore) -> Self {
        self.session_store = Some(store);
        self
    }

    /// Creates a new session and returns its ID.
    pub fn create_session(&mut self, system_prompt: Option<String>) -> Result<String, SessionError> {
        let session_id = match &self.session_store {
            Some(store) => store.create_session(system_prompt.clone())?,
            None => uuid::Uuid::new_v4().to_string(),
        };

        self.current_session_id = Some(session_id.clone());
        self.session = SessionState::with_system_prompt(
            system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
        );

        info!(
            "Created new session {} (endpoint: {})",
            session_id,
            self.config.endpoint.name()
        );

        Ok(session_id)
    }

    /// Loads an existing session from the store.
    pub fn load_session(&mut self, session_id: &str) -> Result<(), SessionError> {
        let state = match &self.session_store {
            Some(store) => store.get_session(session_id)?,
            None => SessionState::new(),
        };

        self.current_session_id = Some(session_id.to_string());
        self.session = state;

        info!("Loaded session {} (endpoint: {})", session_id, self.config.endpoint.name());
        Ok(())
    }

    /// Saves the current session state.
    pub fn save_session(&self) -> Result<(), SessionError> {
        if let (Some(store), Some(sid)) = (&self.session_store, &self.current_session_id) {
            store.update_session(sid, &self.session)?;
        }
        Ok(())
    }

    /// Sends a chat request and returns the unified response.
    pub async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, LmStudioError> {
        // Determine the previous response ID based on the endpoint.
        let previous_response_id = match self.config.endpoint {
            LmStudioEndpoint::Native => self.session.response_id.clone(),
            _ => None,
        };

        // Build the unified request.
        let req = UnifiedChatRequest::from_config(&self.config, user_input, previous_response_id);

        // Convert to endpoint-specific format and send.
        let response = match self.config.endpoint {
            LmStudioEndpoint::Native => self.send_native(&req).await,
            LmStudioEndpoint::Openai => self.send_openai(&req).await,
            LmStudioEndpoint::Anthropic => self.send_anthropic(&req).await,
            LmStudioEndpoint::MistralRsServe => self.send_openai(&req).await,
        }?;

        // Update session state with the response.
        self.session.update_with_response(&response);

        // Save session if using SQLite store.
        if let Err(e) = self.save_session() {
            warn!("Failed to save session: {}", e);
        }

        Ok(response)
    }

    /// Sends a chat request with a system prompt.
    pub async fn chat_with_system(
        &mut self,
        system_prompt: String,
        user_input: String,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let previous_response_id = match self.config.endpoint {
            LmStudioEndpoint::Native => self.session.response_id.clone(),
            _ => None,
        };

        let mut req = UnifiedChatRequest::from_config(&self.config, user_input, previous_response_id);
        req.system_prompt = Some(system_prompt);

        let response = match self.config.endpoint {
            LmStudioEndpoint::Native => self.send_native(&req).await,
            LmStudioEndpoint::Openai => self.send_openai(&req).await,
            LmStudioEndpoint::Anthropic => self.send_anthropic(&req).await,
            LmStudioEndpoint::MistralRsServe => self.send_openai(&req).await,
        }?;

        self.session.update_with_response(&response);

        if let Err(e) = self.save_session() {
            warn!("Failed to save session: {}", e);
        }

        Ok(response)
    }

    /// Extracts tool calls from a response.
    pub fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Vec<ToolCallInfo> {
        response
            .output
            .iter()
            .filter_map(|item| match item {
                UnifiedOutputItem::ToolCall {
                    tool,
                    arguments,
                    output,
                    provider_info,
                } => Some(ToolCallInfo {
                    tool: tool.clone(),
                    arguments: arguments.clone(),
                    output: output.clone(),
                    provider_info: provider_info.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// Extracts text content from a response.
    pub fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        response
            .output
            .iter()
            .filter_map(|item| match item {
                UnifiedOutputItem::Message { content } => Some(content.clone()),
                UnifiedOutputItem::Reasoning { content } => Some(format!("[reasoning] {}", content)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Sends a request to the native endpoint.
    async fn send_native(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = native::to_native_request(req);

        info!(
            "Sending native request to {} (model: {})",
            url, req.model
        );

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("Native endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("Native response: {}", e)))?;

        native::from_native_response(&json).map_err(|e| LmStudioError::ParseError(e.to_string()))
    }

    /// Sends a request to the OpenAI-compatible endpoint.
    async fn send_openai(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = openai::to_openai_request(req);

        info!(
            "Sending OpenAI request to {} (model: {})",
            url, req.model
        );

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("OpenAI endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("OpenAI response: {}", e)))?;

        openai::from_openai_response(&json).map_err(|e| LmStudioError::ParseError(e.to_string()))
    }

    /// Sends a request to the Anthropic-compatible endpoint.
    async fn send_anthropic(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = anthropic::to_anthropic_request(req);

        info!(
            "Sending Anthropic request to {} (model: {})",
            url, req.model
        );

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("Anthropic endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("Anthropic response: {}", e)))?;

        anthropic::from_anthropic_response(&json).map_err(|e| LmStudioError::ParseError(e.to_string()))
    }
}

/// Information about a tool call extracted from a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// The tool name.
    pub tool: String,
    /// The tool arguments.
    pub arguments: serde_json::Value,
    /// The tool output (if available).
    pub output: Option<String>,
    /// The tool provider info.
    pub provider_info: Option<ToolProviderInfo>,
}

/// Errors from LM Studio client operations.
#[derive(Debug, Error)]
pub enum LmStudioError {
    #[error("request failed: {0}")]
    RequestError(String),
    #[error("response parse error: {0}")]
    ParseError(String),
    #[error("HTTP error (status {status}): {body} (endpoint: {endpoint})")]
    HttpError {
        status: u16,
        body: String,
        endpoint: String,
    },
    #[error("session error: {0}")]
    SessionError(String),
}

// ---------------------------------------------------------------------------
// Endpoint auto-detection
// ---------------------------------------------------------------------------

/// Checks which LM Studio endpoints are available by probing them.
///
/// Returns a list of available endpoints. If none are available, returns
/// an error.
///
/// `serve_url` is the mistral.rs serve base URL (default `http://127.0.0.1:8081`).
pub async fn detect_available_endpoints(
    base_url: &str,
    serve_url: Option<&str>,
) -> Result<Vec<LmStudioEndpoint>, LmStudioError> {
    let client = reqwest::Client::new();
    let mut available = Vec::new();

    // Check OpenAI-compatible endpoint.
    let openai_url = format!("{}/v1/chat/completions", base_url);
    if client.get(&openai_url).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
        available.push(LmStudioEndpoint::Openai);
    }

    // Check Anthropic-compatible endpoint.
    let anthropic_url = format!("{}/v1/messages", base_url);
    if client.get(&anthropic_url).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
        available.push(LmStudioEndpoint::Anthropic);
    }

    // Check native endpoint.
    let native_url = format!("{}/api/v1/chat", base_url);
    if client.get(&native_url).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
        available.push(LmStudioEndpoint::Native);
    }

    // Check mistral.rs serve endpoint.
    let mistralrs_url = match serve_url {
        Some(s) => s.to_string(),
        None => std::env::var("MISTRALRS_SERVE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string()),
    };
    let mistralrs_endpoint = format!("{}/v1/chat/completions", mistralrs_url);
    if client.get(&mistralrs_endpoint).send().await.map(|r| r.status().is_success()).unwrap_or(false) {
        available.push(LmStudioEndpoint::MistralRsServe);
    }

    if available.is_empty() {
        Err(LmStudioError::RequestError("No LM Studio endpoints available".to_string()))
    } else {
        Ok(available)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn endpoint_native_path() {
        let ep = LmStudioEndpoint::Native;
        assert_eq!(ep.path(), "/api/v1/chat");
    }

    #[test]
    fn endpoint_openai_path() {
        let ep = LmStudioEndpoint::Openai;
        assert_eq!(ep.path(), "/v1/chat/completions");
    }

    #[test]
    fn endpoint_anthropic_path() {
        let ep = LmStudioEndpoint::Anthropic;
        assert_eq!(ep.path(), "/v1/messages");
    }

    #[test]
    fn endpoint_native_name() {
        let ep = LmStudioEndpoint::Native;
        assert_eq!(ep.name(), "native");
    }

    #[test]
    fn endpoint_openai_name() {
        let ep = LmStudioEndpoint::Openai;
        assert_eq!(ep.name(), "openai-compat");
    }

    #[test]
    fn endpoint_anthropic_name() {
        let ep = LmStudioEndpoint::Anthropic;
        assert_eq!(ep.name(), "anthropic-compat");
    }

    #[test]
    #[serial]
    fn endpoint_from_env_default() {
        unsafe { std::env::remove_var("LM_STUDIO_ENDPOINT"); }
        let ep = LmStudioEndpoint::from_env();
        assert_eq!(ep, LmStudioEndpoint::Openai);
    }

    #[test]
    #[serial]
    fn endpoint_from_env_native() {
        unsafe { std::env::set_var("LM_STUDIO_ENDPOINT", "native"); }
        let ep = LmStudioEndpoint::from_env();
        assert_eq!(ep, LmStudioEndpoint::Native);
        unsafe { std::env::remove_var("LM_STUDIO_ENDPOINT"); }
    }

    #[test]
    #[serial]
    fn endpoint_from_env_openai() {
        unsafe { std::env::set_var("LM_STUDIO_ENDPOINT", "openai"); }
        let ep = LmStudioEndpoint::from_env();
        assert_eq!(ep, LmStudioEndpoint::Openai);
        unsafe { std::env::remove_var("LM_STUDIO_ENDPOINT"); }
    }

    #[test]
    #[serial]
    fn endpoint_from_env_anthropic() {
        unsafe { std::env::set_var("LM_STUDIO_ENDPOINT", "anthropic"); }
        let ep = LmStudioEndpoint::from_env();
        assert_eq!(ep, LmStudioEndpoint::Anthropic);
        unsafe { std::env::remove_var("LM_STUDIO_ENDPOINT"); }
    }

    #[test]
    #[serial]
    fn endpoint_from_env_invalid_defaults_to_openai() {
        unsafe { std::env::set_var("LM_STUDIO_ENDPOINT", "invalid"); }
        let ep = LmStudioEndpoint::from_env();
        assert_eq!(ep, LmStudioEndpoint::Openai);
        unsafe { std::env::remove_var("LM_STUDIO_ENDPOINT"); }
    }

    #[test]
    fn config_from_env_defaults() {
        unsafe {
            std::env::remove_var("LM_STUDIO_URL");
            std::env::remove_var("LM_STUDIO_MODEL");
            std::env::remove_var("LM_STUDIO_CONTEXT_LENGTH");
            std::env::remove_var("LM_STUDIO_TEMPERATURE");
            std::env::remove_var("LM_STUDIO_MAX_OUTPUT_TOKENS");
            std::env::remove_var("LM_API_TOKEN");
        }

        let config = LmStudioConfig::from_env();
        assert_eq!(config.base_url, "http://127.0.0.1:1234");
        assert_eq!(config.default_model, "local-model");
        assert!(config.api_token.is_none());
        assert!(config.default_context_length.is_none());
        assert!(config.default_temperature.is_none());
        assert!(config.default_max_output_tokens.is_none());
    }

    #[test]
    fn config_endpoint_url_constructed() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test-model".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        assert_eq!(
            config.endpoint_url(),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn config_endpoint_url_native() {
        let config = LmStudioConfig {
            base_url: "http://example.com:8080".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Native,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        assert_eq!(config.endpoint_url(), "http://example.com:8080/api/v1/chat");
    }

    #[test]
    fn config_endpoint_url_anthropic() {
        let config = LmStudioConfig {
            base_url: "http://example.com:8080".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Anthropic,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        assert_eq!(config.endpoint_url(), "http://example.com:8080/v1/messages");
    }

    #[test]
    fn message_role_serde_user() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn message_role_serde_system() {
        let role = MessageRole::System;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"system\"");
    }

    #[test]
    fn message_role_serde_assistant() {
        let role = MessageRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn message_role_serde_roundtrip_user() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        let restored: MessageRole = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, role);
    }

    #[test]
    fn unified_message_clone_works() {
        let msg = UnifiedMessage {
            role: MessageRole::User,
            content: "hello".to_string(),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.role, msg.role);
        assert_eq!(cloned.content, msg.content);
    }

    #[test]
    fn unified_chat_request_from_config() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: Some("token".to_string()),
            default_model: "test-model".to_string(),
            default_context_length: Some(4096),
            default_temperature: Some(0.7),
            default_max_output_tokens: Some(1024),
        };
        let req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
        assert_eq!(req.model, "test-model");
        assert_eq!(req.input.content, "hello");
        assert_eq!(req.input.role, MessageRole::User);
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_output_tokens, Some(1024));
        assert_eq!(req.context_length, Some(4096));
        assert_eq!(req.store, Some(true));
    }

    #[test]
    fn reasoning_level_serde_off() {
        let level = ReasoningLevel::Off;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"off\"");
    }

    #[test]
    fn reasoning_level_serde_low() {
        let level = ReasoningLevel::Low;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"low\"");
    }

    #[test]
    fn reasoning_level_serde_medium() {
        let level = ReasoningLevel::Medium;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"medium\"");
    }

    #[test]
    fn reasoning_level_serde_high() {
        let level = ReasoningLevel::High;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn reasoning_level_serde_on() {
        let level = ReasoningLevel::On;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"on\"");
    }

    #[test]
    fn reasoning_level_serde_roundtrip() {
        let level = ReasoningLevel::High;
        let json = serde_json::to_string(&level).unwrap();
        let restored: ReasoningLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, level);
    }

    #[test]
    fn session_state_new_is_empty() {
        let state = SessionState::new();
        assert!(state.response_id.is_none());
        assert!(state.message_history.is_empty());
    }

    #[test]
    fn session_state_with_system_prompt() {
        let state = SessionState::with_system_prompt("You are helpful".to_string());
        assert!(state.response_id.is_none());
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(state.message_history[0].role, MessageRole::System);
        assert_eq!(state.message_history[0].content, "You are helpful");
    }

    #[test]
    fn session_state_add_user_message() {
        let mut state = SessionState::new();
        state.add_user_message("hello".to_string());
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(state.message_history[0].role, MessageRole::User);
        assert_eq!(state.message_history[0].content, "hello");
    }

    #[test]
    fn session_state_has_response_id_false() {
        let state = SessionState::new();
        assert!(!state.has_response_id());
    }

    #[test]
    fn session_state_has_response_id_true() {
        let mut state = SessionState::new();
        state.response_id = Some("resp-123".to_string());
        assert!(state.has_response_id());
    }

    #[test]
    fn session_state_update_with_message_response() {
        let mut state = SessionState::new();
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Message {
                content: "Hello there!".to_string(),
            }],
            stats: None,
            response_id: Some("resp-1".to_string()),
        };
        state.update_with_response(&response);
        assert_eq!(state.response_id, Some("resp-1".to_string()));
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(state.message_history[0].role, MessageRole::Assistant);
        assert_eq!(state.message_history[0].content, "Hello there!");
    }

    #[test]
    fn session_state_update_with_tool_call_response() {
        let mut state = SessionState::new();
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::ToolCall {
                tool: "echo".to_string(),
                arguments: serde_json::json!({"msg": "hi"}),
                output: Some("echoed!".to_string()),
                provider_info: None,
            }],
            stats: None,
            response_id: None,
        };
        state.update_with_response(&response);
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(
            state.message_history[0].content,
            "Tool 'echo' executed: echoed!"
        );
    }

    #[test]
    fn session_state_update_with_reasoning_response() {
        let mut state = SessionState::new();
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Reasoning {
                content: "Let me think about this...".to_string(),
            }],
            stats: None,
            response_id: None,
        };
        state.update_with_response(&response);
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(
            state.message_history[0].content,
            "[reasoning] Let me think about this..."
        );
    }

    #[test]
    fn session_state_update_with_invalid_tool_call_ignored() {
        let mut state = SessionState::new();
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::InvalidToolCall {
                reason: "invalid".to_string(),
                metadata: None,
                tool_name: None,
                provider_info: None,
            }],
            stats: None,
            response_id: None,
        };
        state.update_with_response(&response);
        assert!(state.message_history.is_empty());
    }

    #[test]
    fn session_state_update_preserves_existing_history() {
        let mut state = SessionState::new();
        state.add_user_message("first".to_string());
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Message {
                content: "second".to_string(),
            }],
            stats: None,
            response_id: None,
        };
        state.update_with_response(&response);
        assert_eq!(state.message_history.len(), 2);
        assert_eq!(state.message_history[0].content, "first");
        assert_eq!(state.message_history[1].content, "second");
    }

    #[test]
    fn session_store_new() {
        let store = SessionStore::new("/tmp/sessions.db".to_string());
        assert_eq!(store.db_path, "/tmp/sessions.db");
    }

    #[test]
    fn session_store_create_session_returns_uuid() {
        let store = SessionStore::new(":memory:".to_string());
        let session_id = store.create_session(None).unwrap();
        assert!(!session_id.is_empty());
    }

    #[test]
    fn session_store_create_session_with_system_prompt() {
        let store = SessionStore::new(":memory:".to_string());
        let session_id = store.create_session(Some("You are helpful".to_string())).unwrap();
        assert!(!session_id.is_empty());
    }

    #[test]
    fn session_store_get_session_returns_empty_for_stub() {
        let store = SessionStore::new(":memory:".to_string());
        let state = store.get_session("any-id").unwrap();
        assert!(state.message_history.is_empty());
    }

    #[test]
    fn session_store_update_session_succeeds() {
        let store = SessionStore::new(":memory:".to_string());
        let state = SessionState::new();
        assert!(store.update_session("session-1", &state).is_ok());
    }

    #[test]
    fn session_store_delete_session_succeeds() {
        let store = SessionStore::new(":memory:".to_string());
        assert!(store.delete_session("session-1").is_ok());
    }

    #[test]
    fn native_to_native_request_includes_model() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Native,
            api_token: None,
            default_model: "my-model".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let req = UnifiedChatRequest::from_config(&config, "test".to_string(), None);
        let native = native::to_native_request(&req);
        assert_eq!(native["model"], "my-model");
    }

    #[test]
    fn native_to_native_request_includes_input() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Native,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let req = UnifiedChatRequest::from_config(&config, "hello world".to_string(), None);
        let native = native::to_native_request(&req);
        assert_eq!(native["input"][0]["content"], "hello world");
    }

    #[test]
    fn openai_to_request_includes_messages() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
        let openai = openai::to_openai_request(&req);
        assert_eq!(openai["model"], "test");
        assert_eq!(openai["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn openai_to_request_includes_system_prompt() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let mut req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
        req.system_prompt = Some("You are helpful".to_string());
        let openai = openai::to_openai_request(&req);
        let messages = openai["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
    }

    #[test]
    fn anthropic_to_request_includes_messages() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Anthropic,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
        let anthropic = anthropic::to_anthropic_request(&req);
        assert_eq!(anthropic["model"], "test");
        assert_eq!(anthropic["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn anthropic_to_request_with_system_prompt_prepends() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Anthropic,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let mut req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
        req.system_prompt = Some("You are helpful".to_string());
        let anthropic = anthropic::to_anthropic_request(&req);
        let messages = anthropic["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0]["content"].as_str().unwrap().starts_with("[System prompt:"));
    }

    #[test]
    fn unified_output_item_message_clone() {
        let item = UnifiedOutputItem::Message {
            content: "hello".to_string(),
        };
        let cloned = item.clone();
        match cloned {
            UnifiedOutputItem::Message { content } => assert_eq!(content, "hello"),
            _ => panic!("expected Message variant"),
        }
    }

    #[test]
    fn unified_output_item_tool_call_clone() {
        let item = UnifiedOutputItem::ToolCall {
            tool: "echo".to_string(),
            arguments: serde_json::json!({"x": 1}),
            output: Some("result".to_string()),
            provider_info: None,
        };
        let cloned = item.clone();
        match cloned {
            UnifiedOutputItem::ToolCall { tool, arguments, output, .. } => {
                assert_eq!(tool, "echo");
                assert_eq!(arguments["x"], 1);
                assert_eq!(output, Some("result".to_string()));
            }
            _ => panic!("expected ToolCall variant"),
        }
    }

    #[test]
    fn unified_output_item_reasoning_clone() {
        let item = UnifiedOutputItem::Reasoning {
            content: "thinking...".to_string(),
        };
        let cloned = item.clone();
        match cloned {
            UnifiedOutputItem::Reasoning { content } => assert_eq!(content, "thinking..."),
            _ => panic!("expected Reasoning variant"),
        }
    }

    #[test]
    fn unified_output_item_invalid_tool_call_clone() {
        let item = UnifiedOutputItem::InvalidToolCall {
            reason: "bad".to_string(),
            metadata: None,
            tool_name: None,
            provider_info: None,
        };
        let cloned = item.clone();
        match cloned {
            UnifiedOutputItem::InvalidToolCall { reason, .. } => assert_eq!(reason, "bad"),
            _ => panic!("expected InvalidToolCall variant"),
        }
    }

    #[test]
    fn unified_stats_clone_works() {
        let stats = UnifiedStats {
            input_tokens: 100,
            total_output_tokens: 50,
            reasoning_output_tokens: Some(10),
            tokens_per_second: Some(5.0),
            time_to_first_token_seconds: Some(0.1),
            model_load_time_seconds: Some(1.5),
        };
        let cloned = stats.clone();
        assert_eq!(cloned.input_tokens, 100);
        assert_eq!(cloned.total_output_tokens, 50);
    }

    #[test]
    fn tool_provider_info_serde_roundtrip() {
        let info = ToolProviderInfo {
            provider_type: "plugin".to_string(),
            plugin_id: Some("plugin-1".to_string()),
            server_label: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let restored: ToolProviderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.provider_type, "plugin");
        assert_eq!(restored.plugin_id, Some("plugin-1".to_string()));
    }

    #[test]
    fn lm_studio_error_request_error() {
        let err = LmStudioError::RequestError("connection failed".to_string());
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn lm_studio_error_parse_error() {
        let err = LmStudioError::ParseError("bad response".to_string());
        assert!(err.to_string().contains("bad response"));
    }

    #[test]
    fn lm_studio_error_http_error() {
        let err = LmStudioError::HttpError {
            status: 500,
            body: "internal server error".to_string(),
            endpoint: "test".to_string(),
        };
        assert!(err.to_string().contains("500"));
        assert!(err.to_string().contains("internal server error"));
    }

    #[test]
    fn lm_studio_error_session_error() {
        let err = LmStudioError::SessionError("session lost".to_string());
        assert!(err.to_string().contains("session lost"));
    }

    #[test]
    fn lm_studio_error_debug_format() {
        let err = LmStudioError::RequestError("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("RequestError"));
    }

    #[test]
    fn session_error_not_found() {
        let err = SessionError::NotFound("session-1".to_string());
        assert!(err.to_string().contains("session-1"));
    }

    #[test]
    fn session_error_database() {
        let err = SessionError::Database("disk full".to_string());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn session_error_debug_format() {
        let err = SessionError::NotFound("s1".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn client_from_env_uses_defaults() {
        unsafe {
            std::env::remove_var("LM_STUDIO_URL");
            std::env::remove_var("LM_STUDIO_ENDPOINT");
            std::env::remove_var("LM_STUDIO_MODEL");
            std::env::remove_var("LM_API_TOKEN");
        }

        let client = LmStudioClient::from_env();
        assert_eq!(client.config.base_url, "http://127.0.0.1:1234");
        assert_eq!(client.config.default_model, "local-model");
    }

    #[test]
    fn client_new_with_config() {
        let config = LmStudioConfig {
            base_url: "http://custom:9999".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: Some("secret".to_string()),
            default_model: "custom-model".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let client = LmStudioClient::new(config);
        assert_eq!(client.config.base_url, "http://custom:9999");
        assert_eq!(client.config.default_model, "custom-model");
    }

    #[test]
    fn client_with_session_store() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let store = SessionStore::new("/tmp/sessions.db".to_string());
        let client = LmStudioClient::new(config).with_session_store(store);
        assert!(client.session_store.is_some());
    }

    #[test]
    fn client_create_session_returns_id() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let mut client = LmStudioClient::new(config);
        let session_id = client.create_session(None).unwrap();
        assert!(!session_id.is_empty());
    }

    #[test]
    fn client_create_session_with_system_prompt() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: None,
            default_temperature: None,
            default_max_output_tokens: None,
        };
        let mut client = LmStudioClient::new(config);
        let session_id = client.create_session(Some("You are helpful".to_string())).unwrap();
        assert!(!session_id.is_empty());
        assert_eq!(client.session.message_history.len(), 1);
        assert_eq!(client.session.message_history[0].role, MessageRole::System);
    }

    #[test]
    fn client_load_session_no_store_returns_empty() {
        let mut client = LmStudioClient::from_env();
        let result = client.load_session("any-session");
        assert!(result.is_ok());
    }

    #[test]
    fn client_save_session_no_store_is_ok() {
        let mut client = LmStudioClient::from_env();
        assert!(client.save_session().is_ok());
    }

    #[test]
    fn client_extract_tool_calls_finds_tool_calls() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![
                UnifiedOutputItem::Message {
                    content: "Let me call a tool".to_string(),
                },
                UnifiedOutputItem::ToolCall {
                    tool: "echo".to_string(),
                    arguments: serde_json::json!({"msg": "hello"}),
                    output: None,
                    provider_info: None,
                },
            ],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let calls = client.extract_tool_calls(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool, "echo");
    }

    #[test]
    fn client_extract_tool_calls_empty_when_none() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Message {
                content: "Just text".to_string(),
            }],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let calls = client.extract_tool_calls(&response);
        assert!(calls.is_empty());
    }

    #[test]
    fn client_extract_text_finds_messages() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![
                UnifiedOutputItem::Message {
                    content: "Hello".to_string(),
                },
                UnifiedOutputItem::Message {
                    content: "World".to_string(),
                },
            ],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn client_extract_text_includes_reasoning() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Reasoning {
                content: "thinking...".to_string(),
            }],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.contains("[reasoning]"));
        assert!(text.contains("thinking..."));
    }

    #[test]
    fn client_extract_text_excludes_tool_calls() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::ToolCall {
                tool: "echo".to_string(),
                arguments: serde_json::json!({}),
                output: None,
                provider_info: None,
            }],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.is_empty());
    }

    #[test]
    fn client_extract_text_excludes_invalid_tool_calls() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::InvalidToolCall {
                reason: "bad".to_string(),
                metadata: None,
                tool_name: None,
                provider_info: None,
            }],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.is_empty());
    }

    #[test]
    fn native_from_native_response_parses_message() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [{"type": "message", "content": "hello"}],
            "stats": null,
            "response_id": "resp-1"
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.model_instance_id, "model-1");
        assert_eq!(response.response_id, Some("resp-1".to_string()));
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::Message { content } => assert_eq!(content, "hello"),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn native_from_native_response_parses_tool_call() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [{"type": "tool_call", "tool": "echo", "arguments": {"msg": "hi"}, "output": "echoed", "provider_info": null}],
            "stats": null,
            "response_id": null
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::ToolCall { tool, output, .. } => {
                assert_eq!(tool, "echo");
                assert_eq!(output, &Some("echoed".to_string()));
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn native_from_native_response_parses_reasoning() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [{"type": "reasoning", "content": "thinking..."}],
            "stats": null,
            "response_id": null
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::Reasoning { content } => assert_eq!(content, "thinking..."),
            _ => panic!("expected Reasoning"),
        }
    }

    #[test]
    fn native_from_native_response_parses_invalid_tool_call() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [{"type": "invalid_tool_call", "reason": "bad", "metadata": null, "tool_name": null, "provider_info": null}],
            "stats": null,
            "response_id": null
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::InvalidToolCall { reason, .. } => assert_eq!(reason, "bad"),
            _ => panic!("expected InvalidToolCall"),
        }
    }

    #[test]
    fn native_from_native_response_empty_output() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [],
            "stats": null,
            "response_id": null
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.output.len(), 0);
    }

    #[test]
    fn native_from_native_response_unknown_output_type_ignored() {
        let json = serde_json::json!({
            "model_instance_id": "model-1",
            "output": [{"type": "unknown_type"}],
            "stats": null,
            "response_id": null
        });
        let response = native::from_native_response(&json).unwrap();
        assert_eq!(response.output.len(), 0);
    }

    #[test]
    fn openai_from_response_parses_message() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "choices": [{"message": {"content": "hello", "role": "assistant"}}],
            "usage": null
        });
        let response = openai::from_openai_response(&json).unwrap();
        assert_eq!(response.model_instance_id, "gpt-4");
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::Message { content } => assert_eq!(content, "hello"),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn openai_from_response_parses_tool_call() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": "",
                    "role": "assistant",
                    "tool_calls": [{
                        "function": {"name": "echo", "arguments": "{\"msg\": \"hi\"}"}
                    }]
                }
            }],
            "usage": null
        });
        let response = openai::from_openai_response(&json).unwrap();
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::ToolCall { tool, .. } => assert_eq!(tool, "echo"),
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn openai_from_response_missing_choices_returns_error() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "usage": null
        });
        let result = openai::from_openai_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn openai_from_response_empty_choices_returns_error() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "choices": [],
            "usage": null
        });
        let result = openai::from_openai_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn openai_from_response_missing_message_returns_error() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "choices": [{}],
            "usage": null
        });
        let result = openai::from_openai_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn anthropic_from_response_parses_text() {
        let json = serde_json::json!({
            "model": "claude-3",
            "content": [{"type": "text", "text": "hello"}],
            "usage": null
        });
        let response = anthropic::from_anthropic_response(&json).unwrap();
        assert_eq!(response.model_instance_id, "claude-3");
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::Message { content } => assert_eq!(content, "hello"),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn anthropic_from_response_parses_tool_use() {
        let json = serde_json::json!({
            "model": "claude-3",
            "content": [{"type": "tool_use", "name": "echo", "input": {"msg": "hi"}}],
            "usage": null
        });
        let response = anthropic::from_anthropic_response(&json).unwrap();
        assert_eq!(response.output.len(), 1);
        match &response.output[0] {
            UnifiedOutputItem::ToolCall { tool, .. } => assert_eq!(tool, "echo"),
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn anthropic_from_response_missing_content_returns_error() {
        let json = serde_json::json!({
            "model": "claude-3",
            "usage": null
        });
        let result = anthropic::from_anthropic_response(&json);
        assert!(result.is_err());
    }

    #[test]
    fn unified_chat_request_clone_works() {
        let config = LmStudioConfig {
            base_url: "http://localhost:1234".to_string(),
            serve_base_url: None,
            endpoint: LmStudioEndpoint::Openai,
            api_token: None,
            default_model: "test".to_string(),
            default_context_length: Some(4096),
            default_temperature: Some(0.7),
            default_max_output_tokens: Some(1024),
        };
        let req = UnifiedChatRequest::from_config(&config, "test".to_string(), None);
        let cloned = req.clone();
        assert_eq!(cloned.model, req.model);
        assert_eq!(cloned.input.content, req.input.content);
    }

    #[test]
    fn unified_chat_response_clone_works() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![UnifiedOutputItem::Message {
                content: "hello".to_string(),
            }],
            stats: None,
            response_id: Some("resp-1".to_string()),
        };
        let cloned = response.clone();
        assert_eq!(cloned.model_instance_id, response.model_instance_id);
        assert_eq!(cloned.response_id, response.response_id);
    }

    #[test]
    fn tool_call_info_clone_works() {
        let info = ToolCallInfo {
            tool: "echo".to_string(),
            arguments: serde_json::json!({"x": 1}),
            output: Some("result".to_string()),
            provider_info: None,
        };
        let cloned = info.clone();
        assert_eq!(cloned.tool, info.tool);
        assert_eq!(cloned.output, info.output);
    }

    #[test]
    fn client_extract_text_mixed_output() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![
                UnifiedOutputItem::Message {
                    content: "Hello".to_string(),
                },
                UnifiedOutputItem::ToolCall {
                    tool: "echo".to_string(),
                    arguments: serde_json::json!({}),
                    output: None,
                    provider_info: None,
                },
                UnifiedOutputItem::Reasoning {
                    content: "thinking".to_string(),
                },
            ],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.contains("Hello"));
        assert!(text.contains("[reasoning]"));
        assert!(!text.contains("echo"));
    }

    #[test]
    fn client_extract_text_empty_output_returns_empty() {
        let response = UnifiedChatResponse {
            model_instance_id: "model-1".to_string(),
            output: vec![],
            stats: None,
            response_id: None,
        };
        let client = LmStudioClient::from_env();
        let text = client.extract_text(&response);
        assert!(text.is_empty());
    }
}
