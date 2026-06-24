/// HTTP-based inference backend — routes to a local/self-hosted OpenAI-compatible endpoint.
use async_trait::async_trait;
use reqwest::Client;

use super::backend::{InferenceBackend, InferenceError};

/// OpenAI-compatible chat completion request body.
#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(serde::Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completion response.
#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Option<Vec<ChatChoice>>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(serde::Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

/// HTTP backend that calls an OpenAI-compatible API.
#[derive(Debug, Clone)]
pub struct HttpBackend {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    client: Client,
}

impl HttpBackend {
    pub fn new(endpoint: &str, model: &str, api_key: Option<String>) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl InferenceBackend for HttpBackend {
    async fn infer(&self, prompt: &str) -> Result<String, InferenceError> {
        if self.endpoint.is_empty() {
            return Err(InferenceError::Failed(
                "INFERENCE_ENDPOINT is not set; cannot use http backend".into(),
            ));
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: prompt.to_string(),
            }],
        };

        let mut builder = self.client.post(&self.endpoint).json(&request);

        if let Some(ref key) = self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = builder.send().await.map_err(InferenceError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Failed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let chat_response: ChatResponse = response.json().await?;

        chat_response
            .choices
            .and_then(|mut c| c.pop())
            .and_then(|c| c.message.content)
            .ok_or_else(|| InferenceError::Failed("empty response from model".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_backend_no_endpoint() {
        let backend = HttpBackend::new("", "gpt-4o-mini", None);
        let result = backend.infer("test prompt").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("INFERENCE_ENDPOINT"));
    }
}
