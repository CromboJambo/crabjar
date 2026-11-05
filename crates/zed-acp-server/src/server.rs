//! ACP agent server implementation.
//!
//! Handles incoming requests, manages sessions, and orchestrates agent tasks.

use serde_json::json;

use crabjar_guard::{ExecutionGate, GateContext, GateResult, GuardDb, TrustScore};
use crabjar_lib::knowledge_store::KnowledgeBridge;

use super::types::{AcpRequest, AcpResponse, AcpSession, AcpServerError};

/// The CrabJar version string reported to Zed.
pub const ACP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The ACP agent server that processes requests from Zed.
pub struct AcpAgentServer {
    pub sessions: Vec<AcpSession>,
    pub guard_db: Option<GuardDb>,
    pub knowledge_bridge: Option<KnowledgeBridge>,
    pub initialized: bool,
}

impl AcpAgentServer {
    /// Create a new ACP agent server instance.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            guard_db: None,
            knowledge_bridge: None,
            initialized: false,
        }
    }

    /// Configure the server to use a specific GuardDb for execution gating.
    pub fn with_guard_db(mut self, db: GuardDb) -> Self {
        self.guard_db = Some(db);
        self
    }

    /// Handle an incoming request and produce a response.
    pub async fn handle_request(
        &mut self,
        request: AcpRequest,
    ) -> Result<AcpResponse, AcpServerError> {
        match request {
            AcpRequest::Initialize {
                id,
                protocol_version: _,
                client_capabilities: _,
                client_info: _,
            } => {
                self.initialized = true;
                let response = json!({
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "loadSession": false
                    },
                    "agentInfo": {
                        "name": "crabjar-acp-server",
                        "version": ACP_VERSION
                    }
                });
                Ok(AcpResponse::Result { id, result: response })
            }
            AcpRequest::SessionNew { id, cwd, mcp_servers: _ } => {
                let session = AcpSession::new(cwd);
                self.sessions.push(session.clone());
                let response = json!({
                    "sessionId": session.session_id
                });
                Ok(AcpResponse::Result { id, result: response })
            }
            AcpRequest::SessionLoad { id, session_id, cwd: _ } => {
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
            AcpRequest::SessionPrompt {
                id,
                session_id,
                prompt,
            } => {
                let session = self.sessions.iter().find(|s| s.session_id == session_id);
                match session {
                    Some(_) => {
                        // Extract text from prompt blocks
                        let message = prompt
                            .iter()
                            .filter_map(|block| {
                                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    block
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .map(|s| s.to_string())
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
            AcpRequest::ToolCall {
                id,
                session_id,
                function_name,
                arguments,
            } => {
                let session = self.sessions.iter().find(|s| s.session_id == session_id);
                match session {
                    Some(s) => {
                        let guard_db = self.guard_db.as_ref().ok_or_else(|| {
                            AcpServerError::GuardError("no guard db".into())
                        })?;
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
                            Ok(GateResult::ContextExhausted { used, budget, remaining }) => {
                                Ok(AcpResponse::Result {
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
                                })
                            }
                            Ok(GateResult::OversizedFragment { actual, max }) => {
                                Ok(AcpResponse::Result {
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
                                })
                            }
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
