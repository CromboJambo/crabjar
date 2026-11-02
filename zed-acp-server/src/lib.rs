use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, TrustScore};
use crabjar_lib::knowledge_store::KnowledgeBridge;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub cwd: String,
    pub session_id: String,
    pub initialized: bool,
}

impl AcpSession {
    pub fn new(cwd: String) -> Self {
        Self {
            cwd,
            session_id: uuid::Uuid::new_v4().to_string(),
            initialized: false,
        }
    }
}

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

pub struct AcpAgentServer {
    pub sessions: Vec<AcpSession>,
    pub guard_db: Option<GuardDb>,
    pub knowledge_bridge: Option<KnowledgeBridge>,
    pub initialized: bool,
}

impl AcpAgentServer {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            guard_db: None,
            knowledge_bridge: None,
            initialized: false,
        }
    }

    pub fn with_guard_db(mut self, db: GuardDb) -> Self {
        self.guard_db = Some(db);
        self
    }

    pub async fn handle_request(&mut self, request: AcpRequest) -> Result<AcpResponse, AcpServerError> {
        match request {
            AcpRequest::Initialize { id, protocol_version, client_capabilities, client_info } => {
                self.initialized = true;
                let response = json!({
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "loadSession": false
                    },
                    "agentInfo": {
                        "name": "crabjar-acp-server",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                Ok(AcpResponse::Result { id, result: response })
            }
            AcpRequest::SessionNew { id, cwd, mcp_servers } => {
                let session = AcpSession::new(cwd);
                self.sessions.push(session.clone());
                let response = json!({
                    "sessionId": session.session_id
                });
                Ok(AcpResponse::Result { id, result: response })
            }
            AcpRequest::SessionLoad { id, session_id, cwd } => {
                let found = self.sessions.iter().find(|s| s.session_id == session_id);
                match found {
                    Some(session) => Ok(AcpResponse::Result {
                        id,
                        result: json!({
                            "sessionId": session.session_id,
                            "cwd": session.cwd,
                            "status": "loaded"
                        })
                    }),
                    None => Ok(AcpResponse::Error {
                        id,
                        code: -32602,
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            AcpRequest::SessionClose { id, session_id } => {
                let before = self.sessions.len();
                self.sessions.retain(|s| s.session_id != session_id);
                let after = self.sessions.len();
                if before > after {
                    Ok(AcpResponse::Result {
                        id,
                        result: json!({
                            "sessionId": session_id,
                            "status": "closed"
                        })
                    })
                } else {
                    Ok(AcpResponse::Error {
                        id,
                        code: -32602,
                        message: format!("session not found: {}", session_id),
                    })
                }
            }
            AcpRequest::SessionList { id } => Ok(AcpResponse::Result {
                id,
                result: json!({
                    "sessions": self
                        .sessions
                        .iter()
                        .map(|s| json!({ "sessionId": s.session_id, "cwd": s.cwd }))
                        .collect::<Vec<_>>(),
                    "count": self.sessions.len(),
                }),
            }),
            AcpRequest::SessionPrompt { id, session_id, prompt } => {
                let session = self.sessions.iter().find(|s| s.session_id == session_id);
                match session {
                    Some(_) => {
                        // Extract text from prompt blocks
                        let message = prompt
                            .iter()
                            .filter_map(|block| {
                                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    block.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        let context = self.knowledge_bridge.as_ref().and_then(|bridge| {
                            let tags = ["state-doc", "pattern", "rule"];
                            bridge.query_state_docs(&tags, 50, "").ok()
                        });

                        Ok(AcpResponse::Result {
                            id,
                            result: json!({
                                "sessionId": session_id,
                                "message": message,
                                "context": context,
                                "status": "processed",
                                "stopReason": "end_turn"
                            })
                        })
                    }
                    None => Ok(AcpResponse::Error {
                        id,
                        code: -32602,
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            AcpRequest::ToolCall { id, session_id, function_name, arguments } => {
                let session = self.sessions.iter().find(|s| s.session_id == session_id);
                match session {
                    Some(s) => {
                        let guard_db = self
                            .guard_db
                            .as_ref()
                            .ok_or_else(|| AcpServerError::GuardError("no guard db".into()))?;
                        let gate = ExecutionGate::new(guard_db, false, s.cwd.clone());
                        let command = function_name.clone();
                        let args = arguments
                            .get("args")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .map(|v| v.as_str().unwrap_or("").to_string())
                                    .collect()
                            })
                            .unwrap_or_default();
                        let confidence = arguments
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.9);
                        let source_event_id = arguments
                            .get("provenance_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let gate_result = gate.check(GateContext {
                            action_type: "tool_call",
                            command: &command,
                            args,
                            trust_layer: 3,
                            confidence: TrustScore::new(confidence),
                            source_event_id: source_event_id.as_deref(),
                            can_interrupt: true,
                            pid: None,
                            scope: None,
                            target_scope: None,
                            cross_scope_auth: None,
                            domains: vec![],
                            context_budget: None,
                            context_fragment_tokens: None,
                        });

                        match gate_result {
                            Ok(GateResult::Proceed) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "proceed",
                                    "status": "authorized"
                                })
                            }),
                            Ok(GateResult::Pending) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "pending",
                                    "requires_review": true,
                                    "status": "queued"
                                })
                            }),
                            Ok(GateResult::Interrupted { reason }) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "interrupted",
                                    "reason": reason,
                                    "status": "denied"
                                })
                            }),
                            Ok(GateResult::DryRun) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "dry_run",
                                    "status": "dry_run"
                                })
                            }),
                            Ok(GateResult::Revoked { reason }) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "revoked",
                                    "reason": reason,
                                    "status": "revoked",
                                    "guided_exit": true
                                })
                            }),
                            Ok(GateResult::ContextExhausted { used, budget, remaining }) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "context_exhausted",
                                    "used": used,
                                    "budget": budget,
                                    "remaining": remaining,
                                    "status": "denied"
                                })
                            }),
                            Ok(GateResult::OversizedFragment { actual, max }) => Ok(AcpResponse::Result {
                                id,
                                result: json!({
                                    "sessionId": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "oversized_fragment",
                                    "actual": actual,
                                    "max": max,
                                    "status": "denied"
                                })
                            }),
                            Err(e) => Ok(AcpResponse::Error {
                                id,
                                code: -32603,
                                message: format!("gate error: {}", e),
                            }),
                        }
                    }
                    None => Ok(AcpResponse::Error {
                        id,
                        code: -32602,
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            AcpRequest::Authenticate { id, auth_method } => Ok(AcpResponse::Result {
                id,
                result: json!({
                    "auth_method": auth_method,
                    "status": "authenticated",
                    "requires_api_key": auth_method != "local"
                })
            }),
        }
    }
}

impl Default for AcpAgentServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_initialize() {
        let mut server = AcpAgentServer::new();
        let response = server
            .handle_request(AcpRequest::Initialize {
                id: json!(1),
                protocol_version: 1,
                client_capabilities: json!({}),
                client_info: Some(json!({"name": "test", "version": "1.0"})),
            })
            .await;
        match response {
            Ok(AcpResponse::Result { id, result }) => {
                assert_eq!(id, json!(1));
                assert_eq!(result["protocolVersion"], json!(1));
                assert_eq!(result["agentInfo"]["name"], json!("crabjar-acp-server"));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[tokio::test]
    async fn test_session_new() {
        let mut server = AcpAgentServer::new();
        let response = server
            .handle_request(AcpRequest::SessionNew {
                id: json!(2),
                cwd: "/test/project".to_string(),
                mcp_servers: vec![],
            })
            .await;
        match response {
            Ok(AcpResponse::Result { id, result }) => {
                assert_eq!(id, json!(2));
                assert!(result["sessionId"].is_string());
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[tokio::test]
    async fn test_session_prompt() {
        let mut server = AcpAgentServer::new();
        // Create session first
        let new_response = server
            .handle_request(AcpRequest::SessionNew {
                id: json!(1),
                cwd: "/test/project".to_string(),
                mcp_servers: vec![],
            })
            .await;
        let session_id = match new_response {
            Ok(AcpResponse::Result { result, .. }) => result["sessionId"].as_str().unwrap().to_string(),
            _ => panic!("failed to create session"),
        };

        // Now send prompt
        let response = server
            .handle_request(AcpRequest::SessionPrompt {
                id: json!(2),
                session_id,
                prompt: vec![json!({"type": "text", "text": "test prompt"})],
            })
            .await;
        match response {
            Ok(AcpResponse::Result { result, .. }) => {
                assert_eq!(result["status"], json!("processed"));
                assert_eq!(result["stopReason"], json!("end_turn"));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_tool_call_with_guard() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut server = AcpAgentServer::new().with_guard_db(db);
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Create session first
        let new_response = rt.block_on(server.handle_request(AcpRequest::SessionNew {
            id: json!(1),
            cwd: dir.path().to_string_lossy().into_owned(),
            mcp_servers: vec![],
        }));
        let session_id = match new_response {
            Ok(AcpResponse::Result { result, .. }) => result["sessionId"].as_str().unwrap().to_string(),
            _ => panic!("failed to create session"),
        };

        let response = rt.block_on(server.handle_request(AcpRequest::ToolCall {
            id: json!(2),
            session_id,
            function_name: "echo".to_string(),
            arguments: json!({
                "args": ["hello"],
                "confidence": 0.9,
            }),
        }));
        match response {
            Ok(AcpResponse::Result { result, .. }) => {
                // Gate may return "proceed", "interrupted", or "dry_run" depending on guard config
                let gate_result = result["gate_result"].as_str().unwrap();
                assert!(["proceed", "interrupted", "dry_run"].contains(&gate_result));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_tool_call_without_guard_db() {
        let mut server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Create session first
        let new_response = rt.block_on(server.handle_request(AcpRequest::SessionNew {
            id: json!(1),
            cwd: "/test".to_string(),
            mcp_servers: vec![],
        }));
        let session_id = match new_response {
            Ok(AcpResponse::Result { result, .. }) => result["sessionId"].as_str().unwrap().to_string(),
            _ => panic!("failed to create session"),
        };

        let response = rt.block_on(server.handle_request(AcpRequest::ToolCall {
            id: json!(2),
            session_id,
            function_name: "echo".to_string(),
            arguments: json!({"args": []}),
        }));
        match response {
            Ok(AcpResponse::Error { message, .. }) => {
                assert!(message.contains("guard"));
            }
            Err(AcpServerError::GuardError(msg)) => {
                assert!(msg.contains("guard"));
            }
            Ok(_) => panic!("expected error response"),
            Err(_) => panic!("expected ok with error result"),
        }
    }
}
