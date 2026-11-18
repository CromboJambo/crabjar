//! crabjar-vision — Vision analysis tool for crabjar agents.
//!
//! Provides image understanding capabilities by sending images to LLM vision models
//! via OpenAI-compatible APIs. Supports local file paths and HTTP/HTTPS URLs.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crabjar_vision::{VisionClient, VisionConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = VisionConfig::from_env()?;
//!     let client = VisionClient::new(config);
//!
//!     let result = client.analyze(
//!         "/path/to/image.png",
//!         "What do you see in this image?",
//!     ).await?;
//!
//!     println!("{}", result.analysis);
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};

mod error;
pub use error::{VisionError, VisionResult};

/// Configuration for the vision analysis client.
#[derive(Debug, Clone)]
pub struct VisionConfig {
    /// Base URL of the OpenAI-compatible API endpoint.
    pub base_url: String,
    /// Model name to use for vision analysis.
    pub model: String,
    /// Optional API key / bearer token.
    pub api_key: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum number of tokens for the response.
    pub max_tokens: u32,
}

impl VisionConfig {
    /// Create configuration from environment variables.
    ///
    /// Reads:
    /// - `VISION_API_URL` or `UNSLOTH_STUDIO_URL` (default: http://localhost:8081/v1)
    /// - `VISION_MODEL` or `UNSLOTH_STUDIO_MODEL` (default: "llama-3.2-vision")
    /// - `VISION_API_KEY` or `UNSLOTH_STUDIO_API_KEY`
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("VISION_API_URL")
            .or_else(|_| std::env::var("UNSLOTH_STUDIO_URL"))
            .unwrap_or_else(|_| "http://localhost:8081/v1".to_string());

        let model = std::env::var("VISION_MODEL")
            .or_else(|_| std::env::var("UNSLOTH_STUDIO_MODEL"))
            .unwrap_or_else(|_| "llama-3.2-vision".to_string());

        let api_key = std::env::var("VISION_API_KEY")
            .or_else(|_| std::env::var("UNSLOTH_STUDIO_API_KEY"))
            .ok();

        Ok(Self {
            base_url,
            model,
            api_key,
            timeout_secs: 120,
            max_tokens: 4096,
        })
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set the maximum response tokens.
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = max;
        self
    }
}

/// Result of a vision analysis request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionAnalysis {
    /// The text analysis returned by the model.
    pub analysis: String,
    /// The model that generated the analysis.
    pub model: String,
    /// The source image identifier (path or URL).
    pub source: String,
    /// Number of tokens used in the response (if available).
    pub tokens_used: Option<u32>,
}

/// The vision analysis client.
pub struct VisionClient {
    config: VisionConfig,
    http_client: reqwest::Client,
}

impl VisionClient {
    /// Create a new vision client with the given configuration.
    pub fn new(config: VisionConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http_client,
        }
    }

    /// Analyze an image from a local file path or URL.
    ///
    /// The `source` can be:
    /// - A local file path (e.g., `/path/to/image.png`)
    /// - An HTTP/HTTPS URL (e.g., `https://example.com/image.jpg`)
    pub async fn analyze(&self, source: &str, question: &str) -> VisionResult<VisionAnalysis> {
        info!(source = %source, "Starting vision analysis");

        // Load and encode the image
        let (data_uri, mime_type) = self.load_image(source).await?;

        // Build the OpenAI-compatible vision request
        let request = self.build_vision_request(&data_uri, &mime_type, question)?;

        // Send the request
        let response = self.send_request(&request).await?;

        // Parse the response
        let analysis = self.parse_response(&response, source)?;

        info!(tokens_used = ?analysis.tokens_used, "Vision analysis complete");
        Ok(analysis)
    }

    /// Load an image from a local path or URL and encode as base64 data URI.
    async fn load_image(&self, source: &str) -> VisionResult<(String, String)> {
        let (bytes, mime_type) = if source.starts_with("http://") || source.starts_with("https://") {
            // Download from URL
            info!(url = %source, "Downloading image from URL");
            let response = self
                .http_client
                .get(source)
                .send()
                .await
                .map_err(|e| VisionError::ImageLoadError {
                    image_source: source.to_string(),
                    reason: format!("Failed to download: {}", e),
                })?;

            if !response.status().is_success() {
                return Err(VisionError::ImageLoadError {
                    image_source: source.to_string(),
                    reason: format!("HTTP error: {}", response.status()),
                });
            }

            // Get headers BEFORE consuming the response body
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
                .filter(|s| s.starts_with("image/"))
                .unwrap_or_else(|| detect_mime_from_extension(source));

            let bytes = response.bytes().await.map_err(|e| VisionError::ImageLoadError {
                image_source: source.to_string(),
                reason: format!("Failed to read response body: {}", e),
            })?;

            (bytes.to_vec(), content_type)
        } else {
            // Read from local file
            info!(path = %source, "Reading image from local file");
            let bytes = tokio::fs::read(source).await.map_err(|e| VisionError::ImageLoadError {
                image_source: source.to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

            let mime_type = detect_mime_from_extension(source);
            (bytes, mime_type)
        };

        // Validate image size (warn if very large)
        if bytes.len() > 10 * 1024 * 1024 {
            warn!(size_bytes = bytes.len(), "Image is larger than 10MB");
        }

        // Encode as base64 data URI
        let encoded = BASE64.encode(&bytes);
        let data_uri = format!("data:{};base64,{}", mime_type, encoded);

        Ok((data_uri, mime_type))
    }

    /// Build the OpenAI-compatible vision request payload.
    fn build_vision_request(
        &self,
        data_uri: &str,
        _mime_type: &str,
        question: &str,
    ) -> VisionResult<serde_json::Value> {
        let system_prompt = "You are a helpful vision analysis assistant. Describe what you see in the image clearly and concisely. Focus on answering the user's specific question while providing relevant context about the image content.";

        let request = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": question
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": data_uri
                            }
                        }
                    ]
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": 0.3
        });

        Ok(request)
    }

    /// Send the vision request to the API endpoint.
    async fn send_request(&self, request: &serde_json::Value) -> VisionResult<serde_json::Value> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        info!(url = %url, model = %self.config.model, "Sending vision request");

        let mut builder = self.http_client.post(&url).json(request);

        if let Some(ref api_key) = self.config.api_key {
            builder = builder.bearer_auth(api_key);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| VisionError::RequestError(format!("Failed to send request: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(VisionError::HttpError {
                status: status.as_u16(),
                body,
            });
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| VisionError::ParseError(format!("Failed to parse response JSON: {}", e)))
    }

    /// Parse the API response into a VisionAnalysis result.
    fn parse_response(
        &self,
        response: &serde_json::Value,
        source: &str,
    ) -> VisionResult<VisionAnalysis> {
        let choices = response
            .get("choices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                VisionError::ParseError("Missing 'choices' in API response".to_string())
            })?;

        if choices.is_empty() {
            return Err(VisionError::ParseError(
                "Empty 'choices' array in API response".to_string(),
            ));
        }

        let message = choices[0]
            .get("message")
            .ok_or_else(|| VisionError::ParseError("Missing 'message' in choice".to_string()))?;

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VisionError::ParseError("Missing or non-string 'content' in message".to_string()))?
            .to_string();

        let tokens_used = response
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        Ok(VisionAnalysis {
            analysis: content,
            model: self.config.model.clone(),
            source: source.to_string(),
            tokens_used,
        })
    }
}

/// Detect MIME type from file extension.
fn detect_mime_from_extension(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "webp" => "image/webp".to_string(),
        "bmp" => "image/bmp".to_string(),
        "tiff" | "tif" => "image/tiff".to_string(),
        "svg" => "image/svg+xml".to_string(),
        _ => "image/png".to_string(), // Default to PNG
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Mutex, OnceLock};

    /// Static mutex to serialize env-var tests (they modify global state).
    static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK.get_or_init(Mutex::default).lock().unwrap()
    }

    #[test]
    fn test_detect_mime_from_extension() {
        assert_eq!(detect_mime_from_extension("/path/to/image.png"), "image/png");
        assert_eq!(detect_mime_from_extension("/path/to/photo.jpg"), "image/jpeg");
        assert_eq!(detect_mime_from_extension("/path/to/pic.jpeg"), "image/jpeg");
        assert_eq!(detect_mime_from_extension("/path/to/anim.gif"), "image/gif");
        assert_eq!(detect_mime_from_extension("/path/to/img.webp"), "image/webp");
        assert_eq!(detect_mime_from_extension("/path/to/file"), "image/png"); // Default
    }

    #[test]
    fn test_vision_config_from_env_defaults() {
        let _lock = env_test_lock();
        // Clear any existing env vars
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("UNSLOTH_STUDIO_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("UNSLOTH_STUDIO_MODEL");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://localhost:8081/v1");
        assert_eq!(config.model, "llama-3.2-vision");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_vision_config_with_custom_values() {
        let _lock = env_test_lock();
        unsafe {
            std::env::set_var("VISION_API_URL", "http://custom-host:8080/v1");
            std::env::set_var("VISION_MODEL", "my-vision-model");
            std::env::set_var("VISION_API_KEY", "test-key-123");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://custom-host:8080/v1");
        assert_eq!(config.model, "my-vision-model");
        assert_eq!(config.api_key, Some("test-key-123".to_string()));

        // Cleanup
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("VISION_API_KEY");
        }
    }

    #[test]
    fn test_vision_config_uses_unsloth_studio_fallback() {
        let _lock = env_test_lock();
        // Clear VISION_* vars, set UNSLOTH_STUDIO_* vars
        unsafe {
            std::env::remove_var("VISION_API_URL");
            std::env::remove_var("VISION_MODEL");
            std::env::remove_var("VISION_API_KEY");

            std::env::set_var("UNSLOTH_STUDIO_URL", "http://ftw3:8081/v1");
            std::env::set_var("UNSLOTH_STUDIO_MODEL", "unsloth-vision-model");
            std::env::set_var("UNSLOTH_STUDIO_API_KEY", "unsloth-key");
        }

        let config = VisionConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://ftw3:8081/v1");
        assert_eq!(config.model, "unsloth-vision-model");
        assert_eq!(config.api_key, Some("unsloth-key".to_string()));

        // Cleanup
        unsafe {
            std::env::remove_var("UNSLOTH_STUDIO_URL");
            std::env::remove_var("UNSLOTH_STUDIO_MODEL");
            std::env::remove_var("UNSLOTH_STUDIO_API_KEY");
        }
    }

    #[test]
    fn test_build_vision_request() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient {
            config,
            http_client: reqwest::Client::new(),
        };

        let request = client
            .build_vision_request(
                "data:image/png;base64,TESTDATA",
                "image/png",
                "What is in this image?",
            )
            .unwrap();

        assert_eq!(request["model"], "test-model");
        assert_eq!(request["max_tokens"], 1024);
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);

        // Verify user message has both text and image parts
        let user_msg = &request["messages"][1];
        let content = user_msg["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image_url");
    }

    #[test]
    fn test_parse_response() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient {
            config,
            http_client: reqwest::Client::new(),
        };

        let response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "The image shows a cat sitting on a chair."
                    }
                }
            ],
            "usage": {
                "completion_tokens": 42
            }
        });

        let analysis = client.parse_response(&response, "/test/image.png").unwrap();
        assert_eq!(analysis.analysis, "The image shows a cat sitting on a chair.");
        assert_eq!(analysis.model, "test-model");
        assert_eq!(analysis.source, "/test/image.png");
        assert_eq!(analysis.tokens_used, Some(42));
    }

    #[tokio::test]
    async fn test_analyze_missing_file() {
        let config = VisionConfig {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "test-model".to_string(),
            api_key: None,
            timeout_secs: 60,
            max_tokens: 1024,
        };
        let client = VisionClient::new(config);

        let result = client.analyze("/nonexistent/image.png", "What is this?").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VisionError::ImageLoadError { .. }
        ));
    }
}
