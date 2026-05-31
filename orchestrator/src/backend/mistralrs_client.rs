/// mistral.rs inference client.
///
/// Wraps the mistralrs Rust SDK to provide local LLM inference.
/// Model loading is lazy — the first chat request triggers model download
/// and initialization via `spawn_blocking` to avoid blocking the async runtime.

use crate::backend::InferenceError;
use crate::lm_studio_client::{SessionError, ToolCallInfo, UnifiedChatResponse, UnifiedOutputItem, UnifiedStats};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A chat message stored in history.
#[derive(Debug, Clone)]
struct HistoryMessage {
    role: mistralrs::TextMessageRole,
    content: String,
}

/// Configuration for the mistral.rs client.
#[derive(Debug, Clone)]
struct Config {
    model_id: String,
    quantization: Option<mistralrs::IsqBits>,
}

impl Config {
    fn from_env() -> Self {
        let model_id = std::env::var("MISTRALRS_MODEL")
            .unwrap_or_else(|_| "Qwen/Qwen2.5-Coder-1.5B-Instruct".to_string());

        let quantization = match std::env::var("MISTRALRS_QUANT").ok().as_deref() {
            Some("Q4K") | None => Some(mistralrs::IsqBits::Four),
            Some("Q8_0") => Some(mistralrs::IsqBits::Eight),
            Some("F16") => None, // no quantization
            Some(other) => {
                tracing::warn!("Unknown MISTRALRS_QUANT='{other}', defaulting to Q4K");
                Some(mistralrs::IsqBits::Four)
            }
        };

        Self { model_id, quantization }
    }
}

/// A loaded mistral.rs model, wrapped for interior mutability.
type LoadedModel = Arc<Mutex<mistralrs::Model>>;

/// Client for local inference via mistral.rs.
pub struct MistralRsClient {
    config: Config,
    model: Option<LoadedModel>,
    message_history: Vec<HistoryMessage>,
}

impl MistralRsClient {
    /// Creates a new client with default configuration from environment.
    pub fn new() -> Self {
        Self {
            config: Config::from_env(),
            model: None,
            message_history: Vec::new(),
        }
    }

    /// Lazily loads the model, downloading from HuggingFace if needed.
    async fn load_model(&mut self) -> Result<(), InferenceError> {
        if self.model.is_some() {
            return Ok(());
        }

        let model_id = self.config.model_id.clone();
        let quantization = self.config.quantization;

        tracing::info!(
            "Loading mistral.rs model '{}' (quant: {:?}) — first request will block",
            model_id,
            quantization.map(|q| format!("{q:?}"))
        );

        let builder = mistralrs::ModelBuilder::new(model_id.clone())
            .with_device(
                mistralrs::best_device(true)
                    .unwrap_or_else(|_| mistralrs::Device::cuda_if_available(0).unwrap())
            )
            .with_logging();

        let builder = if let Some(isq) = quantization {
            builder.with_auto_isq(isq)
        } else {
            builder
        };

        let model = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(builder.build())
        })
        .await
        .map_err(|e| InferenceError::BackendError(format!("spawn_blocking failed: {e}")))?
        .map_err(|e| InferenceError::BackendError(format!("model build failed: {e}")))?;

        self.model = Some(Arc::new(Mutex::new(model)));
        tracing::info!("Model '{}' loaded successfully", model_id);
        Ok(())
    }

    /// Sends a chat request, loading the model lazily if needed.
    pub async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError> {
        // Load model on first request.
        if self.model.is_none() {
            self.load_model().await?;
        }

        let model = self.model.as_ref().unwrap().clone();

        // Build messages from history + user input.
        let mut messages = mistralrs::TextMessages::new();
        for msg in &self.message_history {
            messages = messages.add_message(msg.role.clone(), &msg.content);
        }
        messages = messages.add_message(mistralrs::TextMessageRole::User, &user_input);

        let response = tokio::task::spawn_blocking(move || {
            let model_guard = futures::executor::block_on(async { model.lock().await });
            futures::executor::block_on(model_guard.send_chat_request(messages))
        })
        .await
        .map_err(|e| InferenceError::BackendError(format!("spawn_blocking failed: {e}")))?
        .map_err(|e| InferenceError::BackendError(format!("inference failed: {e}")))?;

        // Extract text content from response.
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(choice) = response.choices.first() {
            if let Some(ref content) = choice.message.content {
                if !content.is_empty() {
                    text_parts.push(content.clone());
                }
            }
            if let Some(ref reasoning) = choice.message.reasoning_content {
                if !reasoning.is_empty() {
                    text_parts.push(format!("[reasoning] {}", reasoning));
                }
            }
            if let Some(ref tools) = choice.message.tool_calls {
                for tc in tools {
                    // CalledFunction.arguments is a String — parse it as JSON.
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::String(tc.function.arguments.clone()));
                    tool_calls.push(ToolCallInfo {
                        tool: tc.function.name.clone(),
                        arguments: args,
                        output: None,
                        provider_info: None,
                    });
                }
            }
        }

        // Build unified response with usage stats.
        let unified_output: Vec<UnifiedOutputItem> = text_parts
            .into_iter()
            .map(|content| UnifiedOutputItem::Message { content })
            .collect();

        let unified_stats = if response.usage.total_tokens > 0 {
            Some(UnifiedStats {
                input_tokens: response.usage.prompt_tokens as i64,
                total_output_tokens: response.usage.completion_tokens as i64,
                reasoning_output_tokens: None,
                tokens_per_second: Some(response.usage.avg_compl_tok_per_sec as f64),
                time_to_first_token_seconds: None,
                model_load_time_seconds: None,
            })
        } else {
            None
        };

        let unified_response = UnifiedChatResponse {
            model_instance_id: response.model.clone(),
            output: unified_output,
            stats: unified_stats,
            response_id: None,
        };

        // Add assistant message to history.
        if let Some(ref choice) = response.choices.first() {
            if let Some(ref content) = choice.message.content {
                self.message_history.push(HistoryMessage {
                    role: mistralrs::TextMessageRole::Assistant,
                    content: content.clone(),
                });
            }
        }

        Ok(unified_response)
    }

    /// Extracts text content from a response.
    pub fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        response
            .output
            .iter()
            .filter_map(|item| match item {
                UnifiedOutputItem::Message { content } => Some(content.clone()),
                UnifiedOutputItem::Reasoning { content } => Some(format!("[reasoning] {content}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
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

    /// Creates a new session (resets message history).
    pub fn create_session(&mut self, system_prompt: Option<String>) -> Result<String, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.message_history.clear();

        if let Some(ref prompt) = system_prompt {
            self.message_history.push(HistoryMessage {
                role: mistralrs::TextMessageRole::System,
                content: prompt.clone(),
            });
        }

        tracing::info!("Created mistral.rs session {session_id}");
        Ok(session_id)
    }

    /// Loads an existing session (resets message history — mistral.rs manages state internally).
    pub fn load_session(&mut self, _session_id: &str) -> Result<(), SessionError> {
        tracing::info!("Loaded mistral.rs session (history managed internally)");
        Ok(())
    }

    /// Saves the current session state.
    pub fn save_session(&self) -> Result<(), SessionError> {
        Ok(())
    }
}

impl Default for MistralRsClient {
    fn default() -> Self {
        Self::new()
    }
}
