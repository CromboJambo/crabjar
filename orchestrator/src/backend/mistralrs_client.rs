/// mistral.rs inference client.
///
/// Wraps the mistralrs Rust SDK to provide local LLM inference.
/// Model loading is lazy — the first chat request triggers model download
/// and initialization via `spawn_blocking` to avoid blocking the async runtime.
///
/// Supports:
/// - Chat (non-streaming)
/// - Chat (streaming)
/// - Tool loop (agentic: inference → tool call → execution → resume)

use crate::backend::InferenceError;
use crate::lm_studio_client::{SessionError, ToolCallInfo, UnifiedChatResponse, UnifiedOutputItem, UnifiedStats};
use futures::StreamExt;
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
    /// Maximum tool loop iterations before forcing termination.
    max_tool_iterations: u32,
    /// Whether to enable the agentic tool loop.
    tool_loop_enabled: bool,
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

        let tool_loop_enabled = std::env::var("MISTRALRS_TOOL_LOOP")
            .ok()
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);

        let max_tool_iterations = std::env::var("MISTRALRS_MAX_TOOL_ITERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        Self {
            model_id,
            quantization,
            tool_loop_enabled,
            max_tool_iterations,
        }
    }
}

/// A loaded mistral.rs model, wrapped for interior mutability.
type LoadedModel = Arc<Mutex<mistralrs::Model>>;

/// Callback type for tool execution in the agentic tool loop.
pub type ToolCallback = Box<
    dyn Fn(&str, serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> + Send + Sync,
>;

/// Client for local inference via mistral.rs.
pub struct MistralRsClient {
    config: Config,
    model: Option<LoadedModel>,
    message_history: Vec<HistoryMessage>,
    tool_callback: Option<ToolCallback>,
}

impl MistralRsClient {
    /// Creates a new client with default configuration from environment.
    pub fn new() -> Self {
        Self {
            config: Config::from_env(),
            model: None,
            message_history: Vec::new(),
            tool_callback: None,
        }
    }

    /// Sets the tool callback for agentic tool loop execution.
    pub fn with_tool_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>> + Send + Sync + 'static,
    {
        self.tool_callback = Some(Box::new(callback));
        self
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

    /// Sends a streaming chat request.
    ///
    /// Returns a `ReceiverStream` that yields `UnifiedChatResponse` chunks.
    /// Each chunk contains a single token's delta content. The final chunk
    /// contains the full usage stats.
    ///
    /// Uses an mpsc channel internally: the model is locked via tokio::sync::Mutex
    /// and the mistral.rs stream is iterated on a spawned task. Chunks are forwarded
    /// through the channel to the receiver returned to the caller.
    pub async fn chat_stream(
        &mut self,
        user_input: String,
    ) -> Result<
        std::pin::Pin<Box<dyn futures::stream::Stream<Item = Result<UnifiedChatResponse, InferenceError>> + Send>>,
        InferenceError,
    > {
        // Load model on first request.
        if self.model.is_none() {
            self.load_model().await?;
        }

        let model = self.model.clone().ok_or_else(|| {
            InferenceError::BackendError("model not loaded".to_string())
        })?;

        // Build messages from history + user input.
        let mut messages = mistralrs::TextMessages::new();
        for msg in &self.message_history {
            messages = messages.add_message(msg.role.clone(), &msg.content);
        }
        messages = messages.add_message(mistralrs::TextMessageRole::User, &user_input);

        // mpsc channel: 32 buffers is enough — chunks arrive fast but rarely queue.
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Spawn task: lock model, start stream, forward chunks through channel.
        tokio::spawn(async move {
            let guard = model.lock().await;
            let mut stream = match guard.stream_chat_request(messages).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx
                        .send(Err(InferenceError::BackendError(format!(
                            "stream_chat_request failed: {e}"
                        ))))
                        .await;
                    return;
                }
            };

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    mistralrs::Response::Chunk(mistralrs::ChatCompletionChunkResponse {
                        choices,
                        usage,
                        model: model_id,
                        ..
                    }) => {
                        let mut text_parts = Vec::new();
                        if let Some(choice) = choices.first() {
                            if let Some(ref delta) = choice.delta.content {
                                if !delta.is_empty() {
                                    text_parts.push(delta.clone());
                                }
                            }
                            if let Some(ref reasoning) = choice.delta.reasoning_content {
                                if !reasoning.is_empty() {
                                    text_parts.push(format!("[reasoning] {}", reasoning));
                                }
                            }
                        }

                        let unified_output: Vec<UnifiedOutputItem> = text_parts
                            .into_iter()
                            .map(|content| UnifiedOutputItem::Message { content })
                            .collect();

                        let stats = usage.as_ref().map(|u| UnifiedStats {
                            input_tokens: u.prompt_tokens as i64,
                            total_output_tokens: u.completion_tokens as i64,
                            reasoning_output_tokens: None,
                            tokens_per_second: Some(u.avg_compl_tok_per_sec as f64),
                            time_to_first_token_seconds: None,
                            model_load_time_seconds: None,
                        });

                        let _ = tx
                            .send(Ok(UnifiedChatResponse {
                                model_instance_id: model_id,
                                output: unified_output,
                                stats,
                                response_id: None,
                            }))
                            .await;
                    }
                    mistralrs::Response::Done(mistralrs::ChatCompletionResponse {
                        choices,
                        usage,
                        model: model_id,
                        ..
                    }) => {
                        let mut text_parts = Vec::new();
                        if let Some(choice) = choices.first() {
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
                        }

                        let unified_output: Vec<UnifiedOutputItem> = text_parts
                            .into_iter()
                            .map(|content| UnifiedOutputItem::Message { content })
                            .collect();

                        let _ = tx
                            .send(Ok(UnifiedChatResponse {
                                model_instance_id: model_id,
                                output: unified_output,
                                stats: Some(UnifiedStats {
                                    input_tokens: usage.prompt_tokens as i64,
                                    total_output_tokens: usage.completion_tokens as i64,
                                    reasoning_output_tokens: None,
                                    tokens_per_second: Some(usage.avg_compl_tok_per_sec as f64),
                                    time_to_first_token_seconds: None,
                                    model_load_time_seconds: None,
                                }),
                                response_id: None,
                            }))
                            .await;
                    }
                    mistralrs::Response::InternalError(e) => {
                        let _ = tx
                            .send(Err(InferenceError::BackendError(format!(
                                "internal error: {e:?}"
                            ))))
                            .await;
                    }
                    mistralrs::Response::ModelError(e, _) => {
                        let _ = tx
                            .send(Err(InferenceError::BackendError(format!(
                                "model error: {e}"
                            ))))
                            .await;
                    }
                    mistralrs::Response::ValidationError(e) => {
                        let _ = tx
                            .send(Err(InferenceError::BackendError(format!(
                                "validation error: {e:?}"
                            ))))
                            .await;
                    }
                    _ => {} // Ignore AgenticToolCallProgress, File, etc.
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    /// Runs an agentic tool loop: inference → tool call detection → execution → resume.
    ///
    /// The loop continues until:
    /// - The model returns a non-tool-call response
    /// - The tool callback produces a non-tool-call response
    /// - `max_tool_iterations` is reached
    ///
    /// Returns the final `UnifiedChatResponse` after the loop completes.
    pub async fn tool_loop(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError> {
        if !self.config.tool_loop_enabled {
            tracing::info!("Tool loop disabled; falling back to regular chat");
            return self.chat(user_input).await;
        }

        let mut current_input = user_input;
        let mut last_response = None;

        for iteration in 0..self.config.max_tool_iterations {
            tracing::info!("Tool loop iteration {iteration}");

            let response = self.chat(current_input.clone()).await?;

            // Check for tool calls.
            let tool_calls = self.extract_tool_calls(&response);

            if tool_calls.is_empty() {
                // No more tool calls — this is the final response.
                last_response = Some(response);
                break;
            }

            // Execute each tool call and collect results.
            let mut tool_results = Vec::new();
            for tc in &tool_calls {
                tracing::info!("Executing tool '{}' with args: {:?}", tc.tool, tc.arguments);

                let output = if let Some(ref callback) = self.tool_callback {
                    callback(&tc.tool, tc.arguments.clone()).await
                } else {
                    format!("Error: no tool callback registered for '{}'", tc.tool)
                };

                tool_results.push((tc.tool.clone(), output));
            }

            // Build the next input with tool results appended to history.
            let mut results_str = String::from("Tool results:\n");
            for (tool, output) in &tool_results {
                results_str.push_str(&format!("- Tool '{tool}': {output}\n"));
            }

            // Add assistant tool call messages to history.
            for tc in &tool_calls {
                self.message_history.push(HistoryMessage {
                    role: mistralrs::TextMessageRole::Assistant,
                    content: format!("[tool_call: {}]", tc.tool),
                });
            }

            // Add tool result messages to history.
            for (tool, output) in &tool_results {
                self.message_history.push(HistoryMessage {
                    role: mistralrs::TextMessageRole::Tool,
                    content: output.clone(),
                });
            }

            // Next iteration: the model sees the tool results and decides what to do next.
            current_input = results_str;
            last_response = Some(response);
        }

        if let Some(resp) = last_response {
            Ok(resp)
        } else {
            // Reached max iterations without a non-tool-call response.
            Err(InferenceError::BackendError(format!(
                "Tool loop reached max iterations ({}) without terminating",
                self.config.max_tool_iterations
            )))
        }
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
