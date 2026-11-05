use serde_json::json;
use thiserror::Error;

use crabjar_guard::{ExecutionGate, GateContext, GateResult, TrustScore};

use crate::protocol::{AcpRequest, AcpResponse};

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

pub async fn handle_request(
    server: &mut crate::AcpAgentServer,
    request: AcpRequest,
) -> Result<AcpResponse, AcpServerError> {
    match request {
        AcpRequest::Initialize {
            id,
            protocol_version: _,
            client_capabilities: _,
            client_info: _,
        } => {
            server.initialized = true;
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
            Ok(AcpResponse::Result {
                id,
                result: response,
            })
        }
        AcpRequest::SessionNew {
            id,
            cwd,
            mcp_servers: _,
        } => {
            let session = crate::session::AcpSession::new(cwd);
            server.sessions.push(session.clone());
            let response = json!({ "sessionId": session.session_id });
            Ok(AcpResponse::Result {
                id,
                result: response,
            })
        }
        AcpRequest::SessionLoad {
            id,
            session_id,
            cwd: _,
        } => {
            let found = server.sessions.iter().find(|s| s.session_id == session_id);
            match found {
                Some(session) => Ok(AcpResponse::Result {
                    id,
                    result: json!({
                        "sessionId": session.session_id,
                        "cwd": session.cwd,
                        "status": "loaded"
                    }),
                }),
                None => Ok(AcpResponse::Error {
                    id,
                    code: -32602,
                    message: format!("session not found: {}", session_id),
                }),
            }
        }
        AcpRequest::SessionClose { id, session_id } => {
            let before = server.sessions.len();
            server.sessions.retain(|s| s.session_id != session_id);
            let after = server.sessions.len();
            if before > after {
                Ok(AcpResponse::Result {
                    id,
                    result: json!({ "sessionId": session_id, "status": "closed" }),
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
                "sessions": server
                    .sessions
                    .iter()
                    .map(|s| json!({ "sessionId": s.session_id, "cwd": s.cwd }))
                    .collect::<Vec<_>>(),
                "count": server.sessions.len(),
            }),
        }),
        AcpRequest::SessionPrompt {
            id,
            session_id,
            prompt,
        } => {
            let session = server.sessions.iter().find(|s| s.session_id == session_id);
            match session {
                Some(_) => {
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

                    let context = server.knowledge_bridge.as_ref().and_then(|bridge| {
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
                        }),
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
            let session = server.sessions.iter().find(|s| s.session_id == session_id);
            match session {
                Some(s) => {
                    let guard_db = server
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
                            }),
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
                            }),
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
                            }),
                        }),
                        Ok(GateResult::DryRun) => Ok(AcpResponse::Result {
                            id,
                            result: json!({
                                "sessionId": session_id,
                                "tool": function_name,
                                "arguments": arguments,
                                "gate_result": "dry_run",
                                "status": "dry_run"
                            }),
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
                            }),
                        }),
                        Ok(GateResult::ContextExhausted {
                            used,
                            budget,
                            remaining,
                        }) => Ok(AcpResponse::Result {
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
                            }),
                        }),
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
                                }),
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
            }),
        }),
    }
}
