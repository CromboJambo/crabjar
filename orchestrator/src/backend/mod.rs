/// Unified inference backend abstraction.
///
/// Currently supports LM Studio via the `lm_studio_client` module.

pub use lm_studio_client::UnifiedChatResponse;

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
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LmStudio => write!(f, "lm-studio"),
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
