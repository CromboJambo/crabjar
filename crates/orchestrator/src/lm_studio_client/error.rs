//! Error types and endpoint auto-detection for the Unsloth Studio client.

use super::types::{UnslothStudioEndpoint, ToolProviderInfo};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

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

/// Errors from Unsloth Studio client operations.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum UnslothStudioError {
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

    #[error("prompt error: {0}")]
    PromptError(String),
}

// ---------------------------------------------------------------------------
// Endpoint auto-detection
// ---------------------------------------------------------------------------

/// Checks which Unsloth Studio endpoints are available by probing them.
///
/// Returns a list of available endpoints. If none are available, returns
/// an error.
///
/// `serve_url` is the mistral.rs serve base URL (default `http://127.0.0.1:8081`).
pub async fn detect_available_endpoints(
    base_url: &str,
    serve_url: Option<&str>,
) -> Result<Vec<UnslothStudioEndpoint>, UnslothStudioError> {
    let client = reqwest::Client::new();
    let mut available = Vec::new();

    // Check OpenAI-compatible endpoint.
    let openai_url = format!("{}/v1/chat/completions", base_url);
    if client
        .get(&openai_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
    {
        available.push(UnslothStudioEndpoint::Openai);
    }

    // Check Anthropic-compatible endpoint.
    let anthropic_url = format!("{}/v1/messages", base_url);
    if client
        .get(&anthropic_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
    {
        available.push(UnslothStudioEndpoint::Anthropic);
    }

    // Check native endpoint.
    let native_url = format!("{}/api/v1/chat", base_url);
    if client
        .get(&native_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
    {
        available.push(UnslothStudioEndpoint::Native);
    }

    // Check mistral.rs serve endpoint.
    let mistralrs_url = match serve_url {
        Some(s) => s.to_string(),
        None => std::env::var("MISTRALRS_SERVE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string()),
    };
    let mistralrs_endpoint = format!("{}/v1/chat/completions", mistralrs_url);
    if client
        .get(&mistralrs_endpoint)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
    {
        available.push(UnslothStudioEndpoint::MistralRsServe);
    }

    if available.is_empty() {
        Err(UnslothStudioError::RequestError(
            "No Unsloth Studio endpoints available".to_string(),
        ))
    } else {
        Ok(available)
    }
}
