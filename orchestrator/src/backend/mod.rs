/// Unified inference backend abstraction.
///
/// Allows switching between LM Studio and mistral.rs at runtime via
/// the `INFERENCE_BACKEND` environment variable. Both backends produce
/// the same `UnifiedChatResponse` format so the orchestrator doesn't
/// need to know which backend is in use.
///
/// **Backend selection:**
/// - `INFERENCE_BACKEND=lm-studio` — use LM Studio (default)
/// - `INFERENCE_BACKEND=mistralrs` — use mistral.rs local inference
///
/// **mistral.rs configuration:**
/// - `MISTRALRS_MODEL` — HuggingFace model ID or local GGUF path (default: `Qwen/Qwen2.5-Coder-1.5B-Instruct`)
/// - `MISTRALRS_QUANT` — quantization type: `Q4K`, `Q8_0`, `F16` (default: `Q4K`)

pub use lm_studio_client::UnifiedChatResponse;

mod mistralrs_client;

use crate::lm_studio_client;

// ---------------------------------------------------------------------------
// Trait + enum
// ---------------------------------------------------------------------------

/// Inference backend trait — the unified interface for all LLM backends.
#[async_trait::async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Send a chat request and return the response.
    async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError>;

    /// Extract text content from a response.
    fn extract_text(&self, response: &UnifiedChatResponse) -> String;

    /// Extract tool calls from a response.
    fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Vec<lm_studio_client::ToolCallInfo>;

    /// Create a new session with an optional system prompt.
    fn create_session(&mut self, system_prompt: Option<String>) -> Result<String, lm_studio_client::SessionError>;

    /// Load an existing session by ID.
    fn load_session(&mut self, session_id: &str) -> Result<(), lm_studio_client::SessionError>;

    /// Save the current session state.
    fn save_session(&self) -> Result<(), lm_studio_client::SessionError>;

    /// Returns which backend this is.
    fn kind(&self) -> BackendKind;
}

/// Which inference backend is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    LmStudio,
    MistralRs,
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LmStudio => write!(f, "lm-studio"),
            Self::MistralRs => write!(f, "mistralrs"),
        }
    }
}

/// Errors from inference backend operations.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("backend request failed: {0}")]
    RequestError(String),

    #[error("response parse error: {0}")]
    ParseError(String),

    #[error("backend error: {0}")]
    BackendError(String),

    #[error("session error: {0}")]
    SessionError(String),
}

// ---------------------------------------------------------------------------
// Backend enum — holds either LM Studio or mistral.rs
// ---------------------------------------------------------------------------

/// Unified backend that wraps either LM Studio or mistral.rs.
pub enum Backend {
    LmStudio(lm_studio_client::LmStudioClient),
    MistralRs(mistralrs_client::MistralRsClient),
}

impl Backend {
    /// Creates a new backend from the `INFERENCE_BACKEND` environment variable.
    ///
    /// Defaults to `lm-studio` if the variable is unset or unrecognized.
    pub fn new() -> Self {
        match std::env::var("INFERENCE_BACKEND").ok().as_deref() {
            Some("mistralrs") => {
                tracing::info!("Using mistral.rs inference backend");
                Self::MistralRs(mistralrs_client::MistralRsClient::new())
            }
            _ => {
                tracing::info!("Using LM Studio inference backend (default)");
                Self::LmStudio(lm_studio_client::LmStudioClient::from_env())
            }
        }
    }

    /// Creates a new backend with fallback: tries mistral.rs serve first,
    /// falls back to LM Studio if the serve endpoint is unavailable.
    ///
    /// Probes the mistral.rs serve URL (`MISTRALRS_SERVE_URL`, default `http://127.0.0.1:8081`)
    /// by hitting `/v1/models`. If the probe fails, falls back to LM Studio.
    pub async fn try_new() -> Self {
        // Explicit env var takes priority
        match std::env::var("INFERENCE_BACKEND").ok().as_deref() {
            Some("mistralrs") => {
                tracing::info!("Using mistral.rs inference backend (explicit)");
                return Self::MistralRs(mistralrs_client::MistralRsClient::new());
            }
            Some("lm-studio") | Some("lm_studio") => {
                tracing::info!("Using LM Studio inference backend (explicit)");
                return Self::LmStudio(lm_studio_client::LmStudioClient::from_env());
            }
            _ => {}
        }

        // Probe mistral.rs serve (default port 1234 matches LM Studio default)
        let serve_url = std::env::var("MISTRALRS_SERVE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234".to_string());
        let models_url = format!("{}/v1/models", serve_url);

        match reqwest::get(&models_url).await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("Using mistral.rs inference backend (probe success: {})", models_url);
                Self::MistralRs(mistralrs_client::MistralRsClient::new())
            }
            Ok(resp) => {
                tracing::warn!(
                    "mistral.rs serve returned status {} at {}, falling back to LM Studio",
                    resp.status(),
                    models_url
                );
                Self::LmStudio(lm_studio_client::LmStudioClient::from_env())
            }
            Err(e) => {
                tracing::warn!(
                    "mistral.rs serve probe failed at {}: {}, falling back to LM Studio",
                    models_url,
                    e
                );
                Self::LmStudio(lm_studio_client::LmStudioClient::from_env())
            }
        }
    }

    /// Creates a new LM Studio backend explicitly.
    pub fn lm_studio() -> Self {
        tracing::info!("Using LM Studio inference backend (explicit)");
        Self::LmStudio(lm_studio_client::LmStudioClient::from_env())
    }

    /// Creates a new mistral.rs backend explicitly.
    pub fn mistralrs() -> Self {
        tracing::info!("Using mistral.rs inference backend (explicit)");
        Self::MistralRs(mistralrs_client::MistralRsClient::new())
    }
}

#[async_trait::async_trait]
impl InferenceBackend for Backend {
    async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError> {
        match self {
            Self::LmStudio(client) => {
                client.chat(user_input).await.map_err(|e| InferenceError::RequestError(e.to_string()))
            }
            Self::MistralRs(client) => client.chat(user_input).await,
        }
    }

    fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        match self {
            Self::LmStudio(client) => client.extract_text(response),
            Self::MistralRs(client) => client.extract_text(response),
        }
    }

    fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Vec<lm_studio_client::ToolCallInfo> {
        match self {
            Self::LmStudio(client) => client.extract_tool_calls(response),
            Self::MistralRs(client) => client.extract_tool_calls(response),
        }
    }

    fn create_session(&mut self, system_prompt: Option<String>) -> Result<String, lm_studio_client::SessionError> {
        match self {
            Self::LmStudio(client) => client.create_session(system_prompt),
            Self::MistralRs(client) => client.create_session(system_prompt),
        }
    }

    fn load_session(&mut self, session_id: &str) -> Result<(), lm_studio_client::SessionError> {
        match self {
            Self::LmStudio(client) => client.load_session(session_id),
            Self::MistralRs(client) => client.load_session(session_id),
        }
    }

    fn save_session(&self) -> Result<(), lm_studio_client::SessionError> {
        match self {
            Self::LmStudio(client) => client.save_session(),
            Self::MistralRs(client) => client.save_session(),
        }
    }

    fn kind(&self) -> BackendKind {
        match self {
            Self::LmStudio(_) => BackendKind::LmStudio,
            Self::MistralRs(_) => BackendKind::MistralRs,
        }
    }
}

// ---------------------------------------------------------------------------
// LmStudioClient adapter — implements InferenceBackend for the existing client
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl InferenceBackend for lm_studio_client::LmStudioClient {
    async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError> {
        self.chat(user_input).await.map_err(|e| InferenceError::RequestError(e.to_string()))
    }

    fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        self.extract_text(response)
    }

    fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Vec<lm_studio_client::ToolCallInfo> {
        self.extract_tool_calls(response)
    }

    fn create_session(&mut self, system_prompt: Option<String>) -> Result<String, lm_studio_client::SessionError> {
        self.create_session(system_prompt)
    }

    fn load_session(&mut self, session_id: &str) -> Result<(), lm_studio_client::SessionError> {
        self.load_session(session_id)
    }

    fn save_session(&self) -> Result<(), lm_studio_client::SessionError> {
        self.save_session()
    }

    fn kind(&self) -> BackendKind {
        BackendKind::LmStudio
    }
}
