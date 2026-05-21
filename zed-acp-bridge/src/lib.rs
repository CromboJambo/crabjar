use serde::{Deserialize, Serialize};
use thiserror::Error;
use zed_acp_server::AcpSession;

#[derive(Error, Debug)]
pub enum AcpBridgeError {
    #[error("bridge error: {0}")]
    BridgeError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub function_name: String,
    pub arguments: serde_json::Value,
}

impl ToolSchema {
    pub fn from_function_call(name: &str, args: &str) -> Result<Self, AcpBridgeError> {
        let parsed =
            serde_json::from_str(args).map_err(|e| AcpBridgeError::BridgeError(e.to_string()))?;
        Ok(Self {
            function_name: name.to_string(),
            arguments: parsed,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct AcpBridge {
    pub session: Option<AcpSession>,
    pub events: Vec<TrajectoryEvent>,
}

impl AcpBridge {
    pub fn new() -> Self {
        Self {
            session: None,
            events: Vec::new(),
        }
    }
}

impl Default for AcpBridge {
    fn default() -> Self {
        Self::new()
    }
}
