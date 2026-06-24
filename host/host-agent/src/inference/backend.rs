/// InferenceBackend trait and HeuristicBackend (default heuristic stub).
use async_trait::async_trait;

/// Configuration for the inference backend selection.
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Backend mode: "heuristic" (default) or "http".
    pub mode: String,
    /// HTTP endpoint URL (used when mode == "http").
    pub endpoint: String,
    /// Model name (used when mode == "http").
    pub model: String,
    /// Optional API key for the endpoint.
    pub api_key: Option<String>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            mode: "heuristic".into(),
            endpoint: String::new(),
            model: "gpt-4o-mini".into(),
            api_key: None,
        }
    }
}

impl InferenceConfig {
    /// Build config from environment variables.
    ///
    /// Env vars:
    /// - `INFERENCE_BACKEND` — "heuristic" or "http" (default: "heuristic")
    /// - `INFERENCE_ENDPOINT` — HTTP URL (default: "")
    /// - `INFERENCE_MODEL` — model name (default: "gpt-4o-mini")
    /// - `INFERENCE_API_KEY` — optional API key
    pub fn from_env() -> Self {
        Self {
            mode: std::env::var("INFERENCE_BACKEND").unwrap_or_else(|_| "heuristic".into()),
            endpoint: std::env::var("INFERENCE_ENDPOINT").unwrap_or_default(),
            model: std::env::var("INFERENCE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            api_key: std::env::var("INFERENCE_API_KEY").ok(),
        }
    }
}

/// Error type for inference operations.
#[derive(thiserror::Error, Debug)]
pub enum InferenceError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("response parsing error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("inference failed: {0}")]
    Failed(String),
}

/// Trait for model inference backends.
///
/// Implementations can be deterministic heuristics, local LLM clients,
/// or remote API wrappers. The agent loop stages call this to get
/// model-assisted output; when no model is configured, HeuristicBackend
/// returns deterministic stub results.
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Run inference on the given prompt.
    ///
    /// Returns the model's response text, or an error.
    async fn infer(&self, prompt: &str) -> Result<String, InferenceError>;
}

/// Default heuristic backend — deterministic stub behavior.
///
/// Used when no model is configured or `INFERENCE_BACKEND=heuristic`.
#[derive(Debug, Clone, Default)]
pub struct HeuristicBackend;

#[async_trait]
impl InferenceBackend for HeuristicBackend {
    async fn infer(&self, prompt: &str) -> Result<String, InferenceError> {
        // Deterministic heuristic: return a structured stub response.
        // In practice, this is what all loop stages currently do inline.
        let word_count = prompt.split_whitespace().count();
        Ok(format!(
            "[heuristic] Processed {} words in prompt. No model available; using deterministic stub.",
            word_count
        ))
    }
}
