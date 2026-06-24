/// Inference backend abstraction for the agent loop.
///
/// Provides a swappable model seam: defaults to heuristic (deterministic)
/// behavior, can optionally route to a local/self-hosted OpenAI-compatible
/// endpoint via the `INFERENCE_BACKEND` env var or config flag.
///
/// # Selection
///
/// - `INFERENCE_BACKEND=heuristic` (default) — uses HeuristicBackend
/// - `INFERENCE_BACKEND=http` — uses HttpBackend
/// - `INFERENCE_BACKEND=http` also requires `INFERENCE_ENDPOINT` (URL)
///   and optionally `INFERENCE_MODEL` (model name, defaults to "gpt-4o-mini")
///   and `INFERENCE_API_KEY` (optional, omitted key = no auth header)
mod backend;
mod http_backend;

pub use backend::{InferenceBackend, HeuristicBackend, InferenceConfig};
pub use http_backend::HttpBackend;

/// Create the appropriate backend based on the INFERENCE_BACKEND env var.
pub fn create_backend(config: &InferenceConfig) -> Box<dyn InferenceBackend> {
    match config.mode.as_str() {
        "http" => Box::new(HttpBackend::new(&config.endpoint, &config.model, config.api_key.clone())),
        _ => Box::new(HeuristicBackend),
    }
}
