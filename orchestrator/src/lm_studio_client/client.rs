//! The unified LM Studio client that abstracts endpoint differences.

use crate::endpoints::{anthropic, native, openai};
use crate::session::{SessionError, SessionState, SessionStore};
use crate::types::*;
use tracing::{debug, info, warn};

/// The unified LM Studio client that abstracts endpoint differences.
///
/// It routes requests to the configured endpoint and converts responses
/// to the unified format so the orchestrator doesn't need to know which
/// endpoint it's talking to.
#[derive(Debug)]
pub struct LmStudioClient {
    /// Configuration.
    config: LmStudioConfig,
    /// HTTP client.
    http_client: reqwest::Client,
    /// Session state for stateful continuation.
    session: SessionState,
    /// Session store for persistence.
    session_store: Option<SessionStore>,
    /// Current session ID.
    current_session_id: Option<String>,
}

impl LmStudioClient {
    /// Creates a new client from environment configuration.
    pub fn from_env() -> Self {
        let config = LmStudioConfig::from_env();
        let http_client = reqwest::Client::new();
        let session = SessionState::new();

        Self {
            config,
            http_client,
            session,
            session_store: None,
            current_session_id: None,
        }
    }

    /// Creates a new client with explicit configuration.
    pub fn new(config: LmStudioConfig) -> Self {
        let http_client = reqwest::Client::new();
        let session = SessionState::new();

        Self {
            config,
            http_client,
            session,
            session_store: None,
            current_session_id: None,
        }
    }

    /// Sets the session store for persistence.
    pub fn with_session_store(mut self, store: SessionStore) -> Self {
        self.session_store = Some(store);
        self
    }

    /// Creates a new session and returns its ID.
    pub fn create_session(
        &mut self,
        system_prompt: Option<String>,
    ) -> Result<String, SessionError> {
        let session_id = match &self.session_store {
            Some(store) => store.create_session(system_prompt.clone())?,
            None => uuid::Uuid::new_v4().to_string(),
        };

        self.current_session_id = Some(session_id.clone());
        self.session = SessionState::with_system_prompt(
            system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
        );

        info!(
            "Created new session {} (endpoint: {})",
            session_id,
            self.config.endpoint.name()
        );

        Ok(session_id)
    }

    /// Loads an existing session from the store.
    pub fn load_session(&mut self, session_id: &str) -> Result<(), SessionError> {
        let state = match &self.session_store {
            Some(store) => store.get_session(session_id)?,
            None => SessionState::new(),
        };

        self.current_session_id = Some(session_id.to_string());
        self.session = state;

        info!(
            "Loaded session {} (endpoint: {})",
            session_id,
            self.config.endpoint.name()
        );
        Ok(())
    }

    /// Saves the current session state.
    pub fn save_session(&self) -> Result<(), SessionError> {
        if let (Some(store), Some(sid)) = (&self.session_store, &self.current_session_id) {
            store.update_session(sid, &self.session)?;
        }
        Ok(())
    }

    /// Sends a chat request and returns the unified response.
    pub async fn chat(&mut self, user_input: String) -> Result<UnifiedChatResponse, LmStudioError> {
        // Determine the previous response ID based on the endpoint.
        let previous_response_id = match self.config.endpoint {
            LmStudioEndpoint::Native => self.session.response_id.clone(),
            _ => None,
        };

        // Build the unified request.
        let req = UnifiedChatRequest::from_config(&self.config, user_input, previous_response_id);

        // Convert to endpoint-specific format and send.
        let response = match self.config.endpoint {
            LmStudioEndpoint::Native => self.send_native(&req).await,
            LmStudioEndpoint::Openai => self.send_openai(&req).await,
            LmStudioEndpoint::Anthropic => self.send_anthropic(&req).await,
            LmStudioEndpoint::MistralRsServe => self.send_openai(&req).await,
        }?;

        // Update session state with the response.
        self.session.update_with_response(&response);

        // Save session if using SQLite store.
        if let Err(e) = self.save_session() {
            warn!("Failed to save session: {}", e);
        }

        Ok(response)
    }

    /// Sends a chat request with a system prompt.
    pub async fn chat_with_system(
        &mut self,
        system_prompt: String,
        user_input: String,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let previous_response_id = match self.config.endpoint {
            LmStudioEndpoint::Native => self.session.response_id.clone(),
            _ => None,
        };

        let mut req =
            UnifiedChatRequest::from_config(&self.config, user_input, previous_response_id);
        req.system_prompt = Some(system_prompt);

        let response = match self.config.endpoint {
            LmStudioEndpoint::Native => self.send_native(&req).await,
            LmStudioEndpoint::Openai => self.send_openai(&req).await,
            LmStudioEndpoint::Anthropic => self.send_anthropic(&req).await,
            LmStudioEndpoint::MistralRsServe => self.send_openai(&req).await,
        }?;

        self.session.update_with_response(&response);

        if let Err(e) = self.save_session() {
            warn!("Failed to save session: {}", e);
        }

        Ok(response)
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

    /// Extracts text content from a response.
    pub fn extract_text(&self, response: &UnifiedChatResponse) -> String {
        response
            .output
            .iter()
            .filter_map(|item| match item {
                UnifiedOutputItem::Message { content } => Some(content.clone()),
                UnifiedOutputItem::Reasoning { content } => {
                    Some(format!("[reasoning] {}", content))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Sends a request to the native endpoint.
    async fn send_native(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = native::to_native_request(req);

        info!("Sending native request to {} (model: {})", url, req.model);

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("Native endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("Native response: {}", e)))?;

        native::from_native_response(&json).map_err(|e| LmStudioError::ParseError(e.to_string()))
    }

    /// Sends a request to the OpenAI-compatible endpoint.
    async fn send_openai(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = openai::to_openai_request(req);

        info!("Sending OpenAI request to {} (model: {})", url, req.model);

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("OpenAI endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("OpenAI response: {}", e)))?;

        openai::from_openai_response(&json).map_err(|e| LmStudioError::ParseError(e.to_string()))
    }

    /// Sends a request to the Anthropic-compatible endpoint.
    async fn send_anthropic(
        &self,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse, LmStudioError> {
        let url = self.config.endpoint_url();
        let body = anthropic::to_anthropic_request(req);

        info!(
            "Sending Anthropic request to {} (model: {})",
            url, req.model
        );

        let mut builder = self.http_client.post(&url);

        if let Some(ref token) = self.config.api_token {
            builder = builder.bearer_auth(token);
        }

        let response = builder
            .json(&body)
            .send()
            .await
            .map_err(|e| LmStudioError::RequestError(format!("Anthropic endpoint: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body,
                endpoint: self.config.endpoint.name().to_string(),
            });
        }

        let json = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| LmStudioError::ParseError(format!("Anthropic response: {}", e)))?;

        anthropic::from_anthropic_response(&json)
            .map_err(|e| LmStudioError::ParseError(e.to_string()))
    }
}
