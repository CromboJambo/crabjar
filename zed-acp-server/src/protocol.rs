use serde::{Deserialize, Serialize};

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
