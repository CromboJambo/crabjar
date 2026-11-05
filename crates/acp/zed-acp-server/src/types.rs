//! ACP protocol types: errors, sessions, requests, and responses.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during ACP communication.
#[derive(Error, Debug)]
pub enum AcpServerError {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("protocol error: {0}")]
    ProtocolError(String),
    #[error("knowledge error: {0}")]
    KnowledgeError(String),
    #[error("guard error: {0}")]
    GuardError(String),
}

/// Represents an active agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub cwd: String,
    pub session_id: String,
    pub initialized: bool,
}

impl AcpSession {
    /// Create a new session with the given working directory.
    pub fn new(cwd: String) -> Self {
        Self {
            cwd,
            session_id: uuid::Uuid::new_v4().to_string(),
            initialized: false,
        }
    }
}

/// Requests from Zed to the crabjar agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcpRequest {
    Initialize {
        id: serde_json::Value,
        protocol_version: u32,
        client_capabilities: serde_json::Value,
        client_info: Option<serde_json::Value>,
    },
    SessionNew {
        id: serde_json::Value,
        cwd: String,
        mcp_servers: Vec<serde_json::Value>,
    },
    SessionLoad {
        id: serde_json::Value,
        session_id: String,
        cwd: String,
    },
    SessionClose {
        id: serde_json::Value,
        session_id: String,
    },
    SessionList {
        id: serde_json::Value,
    },
    SessionPrompt {
        id: serde_json::Value,
        session_id: String,
        prompt: Vec<serde_json::Value>,
    },
    ToolCall {
        id: serde_json::Value,
        session_id: String,
        function_name: String,
        arguments: serde_json::Value,
    },
    Authenticate {
        id: serde_json::Value,
        auth_method: String,
    },
}

/// Responses from the crabjar agent to Zed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcpResponse {
    Result {
        id: serde_json::Value,
        result: serde_json::Value,
    },
    Error {
        id: serde_json::Value,
        code: i32,
        message: String,
    },
}
