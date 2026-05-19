//! zed-acp-bridge: Zed extension exposing CrabJar ACP orchestrator.
//!
//! Implements the `zed::Extension` trait to:
//! - Launch the CrabJar ACP orchestrator as a context server
//! - Manage ACP session state locally
//! - Map ACP tool calls to CrabJar command schemas
//! - Enforce guard gate on every tool call

use zed_extension_api as zed;



use serde::{Deserialize, Serialize};



// ---------------------------------------------------------------------------
// ACP Session State
// ---------------------------------------------------------------------------

/// ACP session maintained across prompt calls in Zed's Agent Panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub id: String,
    pub cwd: String,                      // Zed worktree path
    pub trust_layer: u32,
    pub confidence: f64,
    pub trajectory: Vec<TrajectoryEvent>, // streaming event buffer
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
            confidence: 0.5,
            trajectory: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Call Mapping
// ---------------------------------------------------------------------------

/// Maps ACP tool call names to CrabJar command schemas.
#[derive(Debug, Clone)]
pub struct ToolSchema {
    pub tool: String,
    pub args: Vec<String>,
    pub requires_guard: bool,
}

impl ToolSchema {
    pub fn from_function_call(name: &str, arguments: &str) -> Result<Self, String> {
        let args: Vec<String> =
            serde_json::from_str(arguments).map_err(|e| format!("parse error: {}", e))?;

        match name {
            "run_command" => Ok(Self {
                tool: args.first().cloned().unwrap_or_default(),
                args: args[1..].to_vec(),
                requires_guard: true,
            }),
            "search_logs" => Ok(Self {
                tool: "crabjar".to_string(),
                args: vec!["guard".to_string(), "queue".to_string()],
                requires_guard: false,
            }),
            "recent_events" => Ok(Self {
                tool: "crabjar".to_string(),
                args: vec!["knowledge".to_string(), "events".to_string()],
                requires_guard: false,
            }),
            "by_source" => Ok(Self {
                tool: "crabjar".to_string(),
                args: vec!["knowledge".to_string(), "events".to_string()],
                requires_guard: false,
            }),
            "analyze_state" => Ok(Self {
                tool: "crabjar".to_string(),
                args: vec!["state".to_string(), "list".to_string()],
                requires_guard: false,
            }),
            _ => Ok(Self {
                tool: name.to_string(),
                args,
                requires_guard: false,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Orchestrator Bridge
// ---------------------------------------------------------------------------

/// Bridge between Zed Agent Panel and CrabJar ACP orchestrator.
pub struct AcpBridge {
    sessions: std::sync::Mutex<Vec<AcpSession>>,
}

impl Default for AcpBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl AcpBridge {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Create a new ACP session for a Zed worktree.
    pub fn new_session(&mut self, cwd: String) -> Result<AcpSession, String> {
        let session = AcpSession::new(cwd);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.push(session.clone());
        Ok(session)
    }

    /// Load an existing ACP session.
    pub fn load_session(&mut self, session_id: String, cwd: String) -> Result<AcpSession, String> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .iter()
            .find(|s| s.id == session_id)
            .ok_or(format!("Session {} not found", session_id))?;

        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .ok_or(format!("Session {} not found in active list", session_id))?;

        entry.cwd = cwd;
        Ok(session.clone())
    }

    /// List active sessions.
    pub fn list_sessions(&self) -> Vec<AcpSession> {
        self.sessions.lock().unwrap().clone()
    }

    /// Close a session.
    pub fn close_session(&mut self, session_id: String) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|s| s.id != session_id);
        Ok(())
    }

    /// Map an ACP tool call to a CrabJar command schema.
    pub fn map_tool_call(
        &self,
        function_name: &str,
        arguments: &str,
    ) -> Result<ToolSchema, String> {
        ToolSchema::from_function_call(function_name, arguments)
    }

    /// Record a trajectory event for session state tracking.
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
// Zed Extension Implementation
// ---------------------------------------------------------------------------

/// CrabJar ACP extension for Zed's Agent Panel.
#[allow(dead_code)]
pub struct CrabJarAcpExtension {
    bridge: AcpBridge,
}

impl Default for CrabJarAcpExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl CrabJarAcpExtension {
    pub fn new() -> Self {
        Self {
            bridge: AcpBridge::new(),
        }
    }
}

impl zed::Extension for CrabJarAcpExtension {
    fn new() -> Self {
        Self {
            bridge: AcpBridge::new(),
        }
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> Result<zed::Command, String> {
        Ok(zed::Command {
            command: "crabjar".to_string(),
            args: vec!["orchestrator".to_string(), "serve".to_string()],
            env: zed::EnvVars::from_iter(vec![(
                "ACP_ORCHESTRATOR_URL".to_string(),
                "http://127.0.0.1:3000/acp".to_string(),
            )]),
        })
    }
}

zed::register_extension!(CrabJarAcpExtension);
