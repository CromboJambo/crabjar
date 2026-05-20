use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcpServerError {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("protocol error: {0}")]
    ProtocolError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub cwd: String,
    pub session_id: String,
}

impl AcpSession {
    pub fn new(cwd: String) -> Self {
        Self {
            cwd,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZedRequest {
    NewSession { cwd: String },
    LoadSession { session_id: String, cwd: String },
    CloseSession { session_id: String },
    ListSessions,
    Prompt { session_id: String, message: String },
    ToolCall { session_id: String, function_name: String, arguments: serde_json::Value },
    Authenticate { auth_method: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcpResponse {
    Result { value: serde_json::Value },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct AcpAgentServer {
    pub sessions: Vec<AcpSession>,
}

impl AcpAgentServer {
    pub fn new() -> Self {
        Self { sessions: Vec::new() }
    }

    pub fn default() -> Self {
        Self::new()
    }
}
