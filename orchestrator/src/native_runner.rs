//! Native LLM runner client for CrabJar's orchestrator.
//!
//! Bridges the `llm-runner` workspace to the `InferenceBackend` trait.
//! Supports GGUF and Safetensors weight formats with CPU/GPU device selection.

use crate::backend::{BackendKind, InferenceBackend, InferenceError, UnifiedChatResponse};
use crate::lm_studio_client::{
    SessionError, SessionState, ToolCallInfo, UnifiedOutputItem,
};
use crabjar_llm_runner as llm;
use candle_core::DType;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Configuration for the native runner client.
#[derive(Debug, Clone)]
pub struct NativeRunnerConfig {
    /// Path to the GGUF model file.
    pub model_path: PathBuf,
    /// Device backend to use (CPU, CUDA, etc.).
    pub device: String,
    /// Data type for inference (F32, F16).
    pub dtype: String,
    /// Maximum context length in tokens.
    pub max_context_len: usize,
    /// Maximum number of tokens to generate.
    pub max_tokens: usize,
    /// Temperature for sampling.
    pub temperature: f64,
    /// Top-p sampling parameter.
    pub top_p: f64,
}

impl Default for NativeRunnerConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("model.gguf"),
            device: "cpu".to_string(),
            dtype: "F32".to_string(),
            max_context_len: 4096,
            max_tokens: 1024,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}

impl NativeRunnerConfig {
    /// Loads configuration from environment variables.
    pub fn from_env() -> Self {
        let model_path = PathBuf::from(
            std::env::var("NATIVE_MODEL_PATH").unwrap_or_else(|_| "model.gguf".to_string()),
        );
        let device = std::env::var("NATIVE_DEVICE").unwrap_or_else(|_| "cpu".to_string());
        let dtype = std::env::var("NATIVE_DTYPE").unwrap_or_else(|_| "F32".to_string());
        let max_context_len = std::env::var("NATIVE_MAX_CONTEXT_LEN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4096);
        let max_tokens = std::env::var("NATIVE_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024);
        let temperature = std::env::var("NATIVE_TEMPERATURE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.7);
        let top_p = std::env::var("NATIVE_TOP_P")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.9);

        Self {
            model_path,
            device,
            dtype,
            max_context_len,
            max_tokens,
            temperature,
            top_p,
        }
    }
}

/// Native LLM runner client that implements the `InferenceBackend` trait.
pub struct NativeRunnerClient {
    config: NativeRunnerConfig,
    model_manager: llm::ModelManager,
    engine: llm::InferenceEngine,
    tokenizer: llm::Tokenizer,
    session_state: Mutex<SessionState>,
    device_backend: llm::DeviceBackend,
}

impl NativeRunnerClient {
    /// Creates a new native runner client with the given configuration.
    pub async fn new(config: NativeRunnerConfig) -> Result<Self, InferenceError> {
        debug!(
            model_path = %config.model_path.display(),
            device = %config.device,
            dtype = %config.dtype,
            "Native runner: creating client"
        );

        // Initialize device backend
        let mut device_backend = llm::DeviceBackend::new(&config.device);
        device_backend.select().map_err(|e| {
            InferenceError::BackendError(format!("Failed to select device: {}", e))
        })?;

        // Create inference engine
        let dtype = match config.dtype.to_uppercase().as_str() {
            "F16" => DType::F16,
            "F32" => DType::F32,
            "I64" => DType::I64,
            "I32" => DType::I32,
            "U8" => DType::U8,
            _ => DType::F32,
        };
        let engine = llm::InferenceEngine::new(device_backend.device.clone(), dtype);

        // Create model manager (in-memory for now, can be extended to use SQLite)
        let model_manager = llm::ModelManager::new();

        // Create tokenizer
        let tokenizer = llm::Tokenizer::new("gpt2");

        Ok(Self {
            config,
            model_manager,
            engine,
            tokenizer,
            session_state: Mutex::new(SessionState::new()),
            device_backend,
        })
    }

    /// Loads a GGUF model into the runner.
    pub async fn load_model(&self) -> Result<(), InferenceError> {
        if !self.config.model_path.exists() {
            return Err(InferenceError::BackendError(format!(
                "Model file not found: {}",
                self.config.model_path.display()
            )));
        }

        info!(
            model_path = %self.config.model_path.display(),
            "Native runner: loading model"
        );

        // Create model spec
        let spec = llm::ModelSpec {
            name: "native-model".to_string(),
            base_path: self.config.model_path.clone().parent().unwrap_or(&PathBuf::from("/tmp")).to_path_buf(),
            lora_path: None,
            template: None,
            ctx_len: self.config.max_context_len,
            n_threads: None,
        };

        // Register model in the manager
        self.model_manager
            .load_model("native-model".to_string(), spec)
            .await;

        Ok(())
    }

    /// Runs inference on the loaded model.
    pub async fn run_inference(&self, prompt: &str) -> Result<String, InferenceError> {
        debug!(prompt = %prompt, "Native runner: running inference");

        // Encode prompt
        let _tokens = self
            .tokenizer
            .encode(prompt)
            .map_err(|e| InferenceError::BackendError(format!("Failed to encode prompt: {}", e)))?;

        // Run inference (stubbed for now, uses CPU fallback)
        // TODO: Implement actual inference loop using ModelManager and InferenceEngine
        let output = format!("[Native Runner] Inference on '{}' (stubbed)", prompt);

        debug!(output_len = output.len(), "Native runner: inference complete");
        Ok(output)
    }
}

#[async_trait::async_trait]
impl InferenceBackend for NativeRunnerClient {
    async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, InferenceError> {
        // Update session state with user message
        {
            let mut state = self.session_state.lock().await;
            state.add_user_message(user_input.clone());
        }

        // Load model if not already loaded
        // Note: In a real implementation, we'd check if the model is already loaded
        // and only load it if necessary. For now, we assume the model is loaded on startup.

        // Run inference
        let output = self.run_inference(&user_input).await?;

        // Create response
        let response = UnifiedChatResponse {
            model_instance_id: "native-runner".to_string(),
            output: vec![UnifiedOutputItem::Message { content: output }],
            stats: Some(crate::lm_studio_client::UnifiedStats {
                input_tokens: 0,
                total_output_tokens: 0,
                reasoning_output_tokens: None,
                tokens_per_second: None,
                time_to_first_token_seconds: None,
                model_load_time_seconds: None,
            }),
            response_id: None,
        };

        // Update session state with assistant message
        {
            let mut state = self.session_state.lock().await;
            state.update_with_response(&response);
        }

        Ok(response)
    }

    fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        let mut text = String::new();
        for item in &response.output {
            if let UnifiedOutputItem::Message { content } = item {
                text.push_str(content);
            }
        }
        text
    }

    fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Vec<crate::lm_studio_client::ToolCallInfo> {
        let mut calls = Vec::new();
        for item in &response.output {
            if let UnifiedOutputItem::ToolCall {
                tool,
                arguments,
                output,
                provider_info,
            } = item
            {
                calls.push(crate::lm_studio_client::ToolCallInfo {
                    tool: tool.clone(),
                    arguments: arguments.clone(),
                    output: output.clone(),
                    provider_info: provider_info.clone(),
                });
            }
        }
        calls
    }

    fn create_session(&mut self, _system_prompt: Option<String>) -> Result<String, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session_state = SessionState::with_system_prompt(
            _system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
        );
        *self.session_state.blocking_lock() = session_state;
        Ok(session_id)
    }

    fn load_session(&mut self, _session_id: &str) -> Result<(), SessionError> {
        // In a real implementation, we'd load the session from storage
        Ok(())
    }

    fn save_session(&self) -> Result<(), SessionError> {
        // In a real implementation, we'd save the session to storage
        Ok(())
    }

    fn kind(&self) -> BackendKind {
        BackendKind::Native
    }
}
