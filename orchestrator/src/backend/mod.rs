/// Unified inference backend abstraction.
///
/// Currently supports LM Studio via the `lm_studio_client` module.
/// Native inference was moved to the PESTI portable execution substrate.
pub use lm_studio_client::UnifiedChatResponse;

use crate::lm_studio_client;

pub mod metrics;
pub use self::metrics::InferenceMetrics;

// ---------------------------------------------------------------------------
// Trait + enum
// ---------------------------------------------------------------------------

/// Inference backend trait — the unified interface for all LLM backends.
#[async_trait::async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Send a chat request and return the response with inference metrics.
    async fn generate(
        &mut self,
        user_input: String,
    ) -> Result<(UnifiedChatResponse, InferenceMetrics), InferenceError>;

    /// Extract text content from a response.
    fn extract_text(&self, response: &UnifiedChatResponse) -> String;

    /// Extract tool calls from a response.
    fn extract_tool_calls(
        &self,
        response: &UnifiedChatResponse,
    ) -> Vec<lm_studio_client::ToolCallInfo>;

    /// Create a new session with an optional system prompt.
    #[allow(dead_code)]
    fn create_session(
        &mut self,
        system_prompt: Option<String>,
    ) -> Result<String, lm_studio_client::SessionError>;

    /// Load an existing session by ID.
    #[allow(dead_code)]
    fn load_session(&mut self, session_id: &str) -> Result<(), lm_studio_client::SessionError>;

    /// Save the current session state.
    #[allow(dead_code)]
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
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
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
    async fn generate(
        &mut self,
        user_input: String,
    ) -> Result<(UnifiedChatResponse, InferenceMetrics), InferenceError> {
        let start = std::time::Instant::now();
        let response = self
            .chat(user_input)
            .await
            .map_err(|e| InferenceError::RequestError(e.to_string()))?;
        let latency_ms = start.elapsed().as_millis() as u64;

        // Use real token counts from LM Studio API stats when available,
        // fall back to estimation otherwise.
        let (tokens_in, tokens_out) = match response.stats {
            Some(ref s) => (s.input_tokens as u32, s.total_output_tokens as u32),
            None => {
                let content = self.extract_text(&response);
                (0, estimate_tokens(&content))
            }
        };

        // Use the model instance ID from the response if available.
        let model_id = if response.model_instance_id.is_empty() {
            "lm-studio".to_string()
        } else {
            response.model_instance_id.clone()
        };

        let metrics = InferenceMetrics::new(tokens_in, tokens_out, model_id, latency_ms);

        // Calculate cost based on tokens and model (approximate)
        let cost_cents = metrics.estimated_cost_cents();

        Ok((response, metrics))
    }

    fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        self.extract_text(response)
    }

    fn extract_tool_calls(
        &self,
        response: &UnifiedChatResponse,
    ) -> Vec<lm_studio_client::ToolCallInfo> {
        self.extract_tool_calls(response)
    }

    fn create_session(
        &mut self,
        system_prompt: Option<String>,
    ) -> Result<String, lm_studio_client::SessionError> {
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

/// Rough token estimate: ~4 chars per token for English.
fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    ((text.len() as f64) / 4.0).ceil() as u32
}
