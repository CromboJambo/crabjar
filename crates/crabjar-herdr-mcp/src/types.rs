//! Request/response types for herdr MCP tools.

use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListMachinesRequest {
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MachineStatusRequest {
    /// Specific machine label to check (optional).
    #[serde(default)]
    pub machine: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMachineRequest {
    /// SSH target (e.g., "crombo@100.113.106.95").
    pub ssh_target: String,
    /// Label for the machine in herdr.
    pub label: String,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentListRequest {
    /// Filter by session name (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentPromptRequest {
    /// Agent ID to prompt.
    pub agent_id: String,
    /// The prompt text to submit.
    pub prompt: String,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentReadRequest {
    /// Agent ID to read output from.
    pub agent_id: String,
    /// Number of lines to read (default: 50).
    #[serde(default = "default_lines")]
    pub lines: u32,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentWaitRequest {
    /// Agent ID to wait for.
    pub agent_id: String,
    /// State to wait for (e.g., "idle", "busy", "done").
    pub state: String,
    /// Timeout in seconds (default: 300).
    #[serde(default = "default_timeout")]
    pub timeout: u32,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SessionAttachRequest {
    /// Session name to attach to.
    pub session_name: String,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneListRequest {
    /// Filter by session name (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneSplitRequest {
    /// Pane ID to split (optional, defaults to current).
    #[serde(default)]
    pub pane_id: Option<String>,
    /// Split direction: "right" or "down".
    pub direction: String,
    /// Split ratio (0.0-1.0, optional).
    #[serde(default)]
    pub ratio: Option<f64>,
    /// Working directory for new pane (optional).
    #[serde(default)]
    pub cwd: Option<String>,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneFocusRequest {
    /// Target pane ID or neighbor direction (e.g., "left", "right").
    pub target: String,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneRunRequest {
    /// Pane ID to run command in.
    pub pane_id: String,
    /// Command to execute.
    pub command: String,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneReadRequest {
    /// Pane ID to read from.
    pub pane_id: String,
    /// Number of lines to read (optional).
    #[serde(default)]
    pub lines: Option<u32>,
    /// Source: "visible", "recent", "recent-unwrapped" (optional).
    #[serde(default)]
    pub source: Option<String>,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneSendTextRequest {
    /// Pane ID to send text to.
    pub pane_id: String,
    /// Text to send.
    pub text: String,
    /// Send Enter key after text (default: true).
    #[serde(default = "default_true")]
    pub enter: bool,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PaneCloseRequest {
    /// Pane ID to close.
    pub pane_id: String,
    /// Session name to operate in (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

// ─── Session Template Tools ──────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TemplateListRequest {
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TemplateCreateSessionRequest {
    /// Name of the template to use.
    pub template_name: String,
    /// Name for the new session (optional, auto-generated if not provided).
    #[serde(default)]
    pub session_name: Option<String>,
    /// Working directory for the session (optional).
    #[serde(default)]
    pub cwd: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TemplateSaveRequest {
    /// Name to save the template as.
    pub template_name: String,
    /// Current session name to capture layout from (optional).
    #[serde(default)]
    pub session: Option<String>,
    /// Description of the template (optional).
    #[serde(default)]
    pub description: Option<String>,
    /// If true, include raw command output for debugging.
    #[serde(default)]
    pub verbose: bool,
}

pub fn default_lines() -> u32 { 50 }
pub fn default_timeout() -> u32 { 300 }
pub fn default_true() -> bool { true }
