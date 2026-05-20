//! zed-acp-server: stdio ACP agent server implementing Agent Client Protocol via stdin/stdout JSON-RPC.
//!
//! Implements the ACP session lifecycle:
//! - new_session → load_session → close_session
//! - prompt processing with tool call handling
//! - gate enforcement via guard/
//! - tool call mapping via zed-acp-bridge/

use crabjar_guard::{
    ActionStatus, GateResult, ExecutionGate, GateContext, GuardDb, TrustScore, GateConcierge,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

// ---------------------------------------------------------------------------
// ACP Session State
// ---------------------------------------------------------------------------

/// ACP session maintained across prompt calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub id: String,
    pub cwd: String,
    pub trust_layer: u32,
    pub confidence: TrustScore,
    pub trajectory: Vec<TrajectoryEvent>,
    pub created_at: i64,
}

/// A single event in the trajectory buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    pub timestamp: i64,
    pub source: String,
    pub content: String,
    pub preview: String,
}

impl AcpSession {
    pub fn new(cwd: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cwd,
            trust_layer: 2,
            confidence: TrustScore::new(0.5),
            trajectory: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

// ---------------------------------------------------------------------------
// ACP Protocol Requests
// ---------------------------------------------------------------------------

/// ACP session lifecycle request.
#[derive(Debug, Deserialize)]
pub enum AcpRequest {
    NewSession { cwd: String },
    LoadSession { session_id: String, cwd: String },
    CloseSession { session_id: String },
    ListSessions,
    Prompt { session_id: String, message: String },
    ToolCall { session_id: String, function_name: String, arguments: String },
    Authenticate { auth_method: String },
}

/// ACP response.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcpResponse {
    Session { session: AcpSession },
    Sessions { sessions: Vec<AcpSession> },
    Prompt { content: String },
    ToolCall { result: String },
    Auth { authenticated: bool },
    Error { error: String },
}

// ---------------------------------------------------------------------------
// Orchestrator Bridge
// ---------------------------------------------------------------------------

/// Bridge between ACP session and CrabJar execution.
pub struct AcpBridge {
    sessions: std::sync::Mutex<Vec<AcpSession>>,
    guard_db: GuardDb,
}

impl Default for AcpBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl AcpBridge {
    pub fn new() -> Self {
        let guard_db = GuardDb::open(":memory:").unwrap_or_else(|_| {
            warn!("Failed to open guard DB, using in-memory fallback");
            GuardDb::open(":memory:").unwrap()
        });

        Self {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        }
    }

    pub fn new_session(&mut self, cwd: String) -> Result<AcpSession, String> {
        let session = AcpSession::new(cwd);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.push(session.clone());
        Ok(session)
    }

    pub fn load_session(&mut self, session_id: String, cwd: String) -> Result<AcpSession, String> {
        let mut sessions = self.sessions.lock().unwrap();
        let mut entry = sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .cloned()
            .ok_or(format!("Session {} not found", session_id))?;

        entry.cwd = cwd;
        Ok(entry)
    }

    pub fn close_session(&mut self, session_id: String) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|s| s.id != session_id);
        Ok(())
    }

    pub fn list_sessions(&self) -> Vec<AcpSession> {
        self.sessions.lock().unwrap().clone()
    }

    /// Gate check for a tool call.
    pub fn gate_check(
        &self,
        session_id: String,
        command: &str,
        args: &[String],
    ) -> Result<(GateResult, ActionStatus), String> {
        let session_data = {
            let sessions = self.sessions.lock().unwrap();
            sessions
                .iter()
                .find(|s| s.id == session_id)
                .cloned()
                .ok_or(format!("Session {} not found", session_id))?
        };

        let gate = ExecutionGate::new(&self.guard_db, false, "/tmp");

        let gate_result = gate
            .check(GateContext {
                action_type: "tool_call",
                command,
                args: args.to_vec(),
                trust_layer: session_data.trust_layer,
                confidence: session_data.confidence,
                source_event_id: Some("acp-tc"),
                can_interrupt: true,
            })
            .map_err(|e| e.to_string())?;

        let _concierge = GateConcierge { db: None };
        let status = match gate_result {
            GateResult::Proceed => ActionStatus::TrustApproved,
            GateResult::Pending => ActionStatus::Pending,
            GateResult::Interrupted { .. } => ActionStatus::Denied,
            GateResult::DryRun => ActionStatus::Denied,
        };

        Ok((gate_result, status))
    }

    /// Record a trajectory event.
    pub fn record_event(&mut self, session_id: String, event: TrajectoryEvent) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .ok_or(format!("Session {} not found", session_id))?;

        entry.trajectory.push(event);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Agent Server
// ---------------------------------------------------------------------------

/// ACP agent server running via stdin/stdout.
pub struct AcpAgentServer {
    bridge: AcpBridge,
    orchestrator_url: String,
}

impl Default for AcpAgentServer {
    fn default() -> Self {
        Self::new()
    }
}

impl AcpAgentServer {
    pub fn new() -> Self {
        let orchestrator_url = std::env::var("ACP_ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3000/acp".to_string());

        Self {
            bridge: AcpBridge::new(),
            orchestrator_url,
        }
    }

    /// Handle an ACP request.
    pub async fn handle_request(&mut self, request: AcpRequest) -> AcpResponse {
        match request {
            AcpRequest::NewSession { cwd } => {
                match self.bridge.new_session(cwd) {
                    Ok(session) => AcpResponse::Session { session },
                    Err(e) => AcpResponse::Error { error: e },
                }
            }
            AcpRequest::LoadSession { session_id, cwd } => {
                match self.bridge.load_session(session_id, cwd) {
                    Ok(session) => AcpResponse::Session { session },
                    Err(e) => AcpResponse::Error { error: e },
                }
            }
            AcpRequest::CloseSession { session_id } => {
                match self.bridge.close_session(session_id) {
                    Ok(()) => AcpResponse::Sessions { sessions: self.bridge.list_sessions() },
                    Err(e) => AcpResponse::Error { error: e },
                }
            }
            AcpRequest::ListSessions => {
                AcpResponse::Sessions { sessions: self.bridge.list_sessions() }
            }
            AcpRequest::Prompt { session_id, message } => {
                let client = reqwest::Client::new();
                let response = client
                    .post(format!("{}/prompt", self.orchestrator_url))
                    .json(&serde_json::json!({
                        "message": message,
                    }))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to connect to orchestrator: {}", e));

                match response {
                    Ok(resp) => {
                        let body = resp
                            .text()
                            .await
                            .map_err(|e| format!("Failed to parse response: {}", e));

                        match body {
                            Ok(content) => {
                                let _ = self.bridge.record_event(
                                    session_id,
                                    TrajectoryEvent {
                                        timestamp: chrono::Utc::now().timestamp(),
                                        source: "user_prompt".to_string(),
                                        content: message.clone(),
                                        preview: message.chars().take(200).collect(),
                                    },
                                );
                                AcpResponse::Prompt { content }
                            }
                            Err(e) => AcpResponse::Error { error: e },
                        }
                    }
                    Err(e) => AcpResponse::Error { error: e },
                }
            }
            AcpRequest::ToolCall { session_id, function_name, arguments } => {
                let args: Vec<String> =
                    serde_json::from_str(&arguments).unwrap_or_else(|e| {
                        warn!("Failed to parse tool arguments: {}", e);
                        vec![]
                    });

                let (gate_result, status) = self
                    .bridge
                    .gate_check(session_id.clone(), &function_name, &args)
                    .unwrap_or_else(|e| {
                        warn!("Gate check failed: {}", e);
                        (GateResult::Interrupted { reason: "gate error".to_string() }, ActionStatus::Denied)
                    });

                match status {
                    ActionStatus::Denied => AcpResponse::Error {
                        error: format!("Tool call denied by gate: {:?}", gate_result),
                    },
                    ActionStatus::Pending => AcpResponse::Error {
                        error: "Tool call pending — queued for review".to_string(),
                    },
                    ActionStatus::TrustApproved => {
                        let client = reqwest::Client::new();
                        let response = client
                            .post(format!("{}/run", self.orchestrator_url))
                            .json(&serde_json::json!({
                                "tool": function_name,
                                "args": args,
                            }))
                            .send()
                            .await
                            .map_err(|e| format!("Failed to execute via orchestrator: {}", e));

                        match response {
                            Ok(resp) => {
                                let body = resp
                                    .text()
                                    .await
                                    .map_err(|e| format!("Failed to parse response: {}", e));

                                match body {
                                    Ok(content) => {
                                        let _ = self.bridge.record_event(
                                            session_id,
                                            TrajectoryEvent {
                                                timestamp: chrono::Utc::now().timestamp(),
                                                source: "tool_call".to_string(),
                                                content: content.clone(),
                                                preview: content.chars().take(200).collect(),
                                            },
                                        );
                                        AcpResponse::ToolCall { result: content }
                                    }
                                    Err(e) => AcpResponse::Error { error: e },
                                }
                            }
                            Err(e) => AcpResponse::Error { error: e },
                        }
                    }
                    ActionStatus::Executed | ActionStatus::Interrupted => {
                        AcpResponse::Error {
                            error: "Status not handled".to_string(),
                        }
                    }
                }
            }
            AcpRequest::Authenticate { auth_method: _ } => {
                AcpResponse::Auth { authenticated: true }
            }
        }
    }
}


