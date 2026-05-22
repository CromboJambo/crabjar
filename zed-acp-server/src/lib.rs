use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crabjar_lib::knowledge_store::KnowledgeBridge;
use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, TrustScore};

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
    NewSession {
        cwd: String,
    },
    LoadSession {
        session_id: String,
        cwd: String,
    },
    CloseSession {
        session_id: String,
    },
    ListSessions,
    Prompt {
        session_id: String,
        message: String,
    },
    ToolCall {
        session_id: String,
        function_name: String,
        arguments: serde_json::Value,
    },
    Authenticate {
        auth_method: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcpResponse {
    Result { value: serde_json::Value },
    Error { message: String },
}

pub struct AcpAgentServer {
    pub sessions: Vec<AcpSession>,
    pub guard_db: Option<GuardDb>,
    pub knowledge_bridge: Option<KnowledgeBridge>,
}

impl AcpAgentServer {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            guard_db: None,
            knowledge_bridge: None,
        }
    }

    pub fn with_guard_db(mut self, db: GuardDb) -> Self {
        self.guard_db = Some(db);
        self
    }

    pub async fn handle_request(&mut self, request: ZedRequest) -> Result<AcpResponse, AcpServerError> {
        match request {
            ZedRequest::NewSession { cwd } => {
                let session = AcpSession::new(cwd);
                Ok(AcpResponse::Result {
                    value: json!({
                        "session_id": session.session_id,
                        "cwd": session.cwd,
                        "status": "created",
                    }),
                })
            }
            ZedRequest::LoadSession { session_id, cwd: _ } => {
                let found = self.sessions.iter().find(|s| s.session_id == session_id);
                match found {
                    Some(session) => Ok(AcpResponse::Result {
                        value: json!({
                            "session_id": session.session_id,
                            "cwd": session.cwd,
                            "status": "loaded",
                        }),
                    }),
                    None => Ok(AcpResponse::Error {
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            ZedRequest::CloseSession { session_id } => {
                let before = self.sessions.len();
                self.sessions.retain(|s| s.session_id != session_id);
                let after = self.sessions.len();
                if before > after {
                    Ok(AcpResponse::Result {
                        value: json!({
                            "session_id": session_id,
                            "status": "closed",
                        }),
                    })
                } else {
                    Ok(AcpResponse::Error {
                        message: format!("session not found: {}", session_id),
                    })
                }
            }
            ZedRequest::ListSessions => {
                Ok(AcpResponse::Result {
                    value: json!({
                        "sessions": self
                            .sessions
                            .iter()
                            .map(|s| json!({ "session_id": s.session_id, "cwd": s.cwd }))
                            .collect::<Vec<_>>(),
                        "count": self.sessions.len(),
                    }),
                })
            }
            ZedRequest::Prompt { session_id, message } => {
                let session = self.sessions.iter().find(|s| s.session_id == session_id);
                match session {
                    Some(_) => {
                        let context = self
                            .knowledge_bridge
                            .as_ref()
                            .and_then(|bridge| {
                                let tags = ["state-doc", "pattern", "rule"];
                                bridge.query_state_docs(&tags, 50, "").ok()
                            });
                        Ok(AcpResponse::Result {
                            value: json!({
                                "session_id": session_id,
                                "message": message,
                                "context": context,
                                "status": "processed",
                                }),
                            })
                    }
                    None => Ok(AcpResponse::Error {
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            ZedRequest::ToolCall {
                session_id,
                function_name,
                arguments,
            } => {
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
                            .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
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
                        });

                        match gate_result {
                            Ok(GateResult::Proceed) => Ok(AcpResponse::Result {
                                value: json!({
                                    "session_id": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "proceed",
                                    "status": "authorized",
                                }),
                            }),
                            Ok(GateResult::Pending) => Ok(AcpResponse::Result {
                                value: json!({
                                    "session_id": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "pending",
                                    "requires_review": true,
                                    "status": "queued",
                                }),
                            }),
                            Ok(GateResult::Interrupted { reason }) => Ok(AcpResponse::Result {
                                value: json!({
                                    "session_id": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "interrupted",
                                    "reason": reason,
                                    "status": "denied",
                                }),
                            }),
                            Ok(GateResult::DryRun) => Ok(AcpResponse::Result {
                                value: json!({
                                    "session_id": session_id,
                                    "tool": function_name,
                                    "arguments": arguments,
                                    "gate_result": "dry_run",
                                    "status": "dry_run",
                                }),
                            }),
                            Err(e) => Ok(AcpResponse::Error {
                                message: format!("gate error: {}", e),
                            }),
                        }
                    }
                    None => Ok(AcpResponse::Error {
                        message: format!("session not found: {}", session_id),
                    }),
                }
            }
            ZedRequest::Authenticate { auth_method } => {
                Ok(AcpResponse::Result {
                    value: json!({
                        "auth_method": auth_method,
                        "status": "authenticated",
                        "requires_api_key": auth_method != "local",
                    }),
                })
            }
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

    #[test]
    fn test_new_session_creates_id() {
        let server = AcpAgentServer::new();
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(ZedRequest::NewSession {
            cwd: "/test/project".to_string(),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert!(value["session_id"].as_str().is_some());
                assert_eq!(value["cwd"].as_str(), Some("/test/project"));
                assert_eq!(value["status"].as_str(), Some("created"));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_load_session_found() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: "/test/project".to_string(),
        }));
        let response = rt.block_on(server.handle_request(ZedRequest::LoadSession {
            session_id: server.sessions[0].session_id.clone(),
            cwd: "/test/project".to_string(),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["status"].as_str(), Some("loaded"));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_load_session_not_found() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(server.handle_request(ZedRequest::LoadSession {
            session_id: "nonexistent".to_string(),
            cwd: "/test/project".to_string(),
        }));
        match response {
            Ok(AcpResponse::Error { message }) => {
                assert!(message.contains("nonexistent"));
            }
            Ok(_) => panic!("unexpected result response"),
            Err(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_close_session() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: "/test/project".to_string(),
        }));
        let session_id = server.sessions[0].session_id.clone();
        let response = rt.block_on(server.handle_request(ZedRequest::CloseSession { session_id }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["status"].as_str(), Some("closed"));
            }
            Ok(_) => panic!("unexpected error response"),
            Err(_) => panic!("expected result"),
        }
        assert!(server.sessions.is_empty());
    }

    #[test]
    fn test_list_sessions() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: "/test/project".to_string(),
        }));
        let response = rt.block_on(server.handle_request(ZedRequest::ListSessions));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["count"].as_i64(), Some(1));
            }
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_prompt_with_session() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: "/test/project".to_string(),
        }));
        let session_id = server.sessions[0].session_id.clone();
        let response = rt.block_on(server.handle_request(ZedRequest::Prompt {
            session_id,
            message: "test prompt".to_string(),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["status"].as_str(), Some("processed"));
            }
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_prompt_without_session() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(server.handle_request(ZedRequest::Prompt {
            session_id: "nonexistent".to_string(),
            message: "test prompt".to_string(),
        }));
        match response {
            Ok(AcpResponse::Error { message }) => {
                assert!(message.contains("nonexistent"));
            }
            Err(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_authenticate() {
        let server = AcpAgentServer::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(server.handle_request(ZedRequest::Authenticate {
            auth_method: "api_key".to_string(),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["status"].as_str(), Some("authenticated"));
                assert_eq!(value["requires_api_key"].as_bool(), Some(true));
            }
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_tool_call_with_guard() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let server = AcpAgentServer::new().with_guard_db(db);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: dir.path().to_string_lossy().into_owned(),
        }));
        let session_id = server.sessions[0].session_id.clone();

        let response = rt.block_on(server.handle_request(ZedRequest::ToolCall {
            session_id,
            function_name: "echo".to_string(),
            arguments: json!({
                "args": ["hello"],
                "confidence": 0.9,
            }),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["gate_result"].as_str(), Some("proceed"));
            }
            Err(_) => panic!("expected result"),
        }
    }

    #[test]
    fn test_tool_call_high_risk_blocked() {
        let dir = tempdir().unwrap();
        let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let server = AcpAgentServer::new().with_guard_db(db);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(server.handle_request(ZedRequest::NewSession {
            cwd: dir.path().to_string_lossy().into_owned(),
        }));
        let session_id = server.sessions[0].session_id.clone();

        let response = rt.block_on(server.handle_request(ZedRequest::ToolCall {
            session_id,
            function_name: "rm".to_string(),
            arguments: json!({
                "args": ["-rf", "/tmp"],
                "confidence": 0.9,
            }),
        }));
        match response {
            Ok(AcpResponse::Result { value }) => {
                assert_eq!(value["gate_result"].as_str(), Some("interrupted"));
            }
            Err(_) => panic!("expected result"),
        }
    }
}
