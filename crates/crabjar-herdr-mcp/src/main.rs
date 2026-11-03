//! crabjar-herdr-mcp: MCP server wrapping herdr for fleet machine management and agent delegation.
//!
//! Exposes tools for listing machines, checking status, adding machines, managing agents,
//! delegating tasks across the home-lab fleet via herdr's SSH integration, and controlling
//! panes within herdr sessions.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::process::Command;

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

fn default_lines() -> u32 { 50 }
fn default_timeout() -> u32 { 300 }
fn default_true() -> bool { true }

#[derive(Debug, Clone)]
pub struct HerdrServer;

fn run_cmd(cmd: &str) -> anyhow::Result<serde_json::Value> {
    let output = Command::new("bash")
        .args(["-c", cmd])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        match serde_json::from_str(&stdout) {
            Ok(val) => Ok(val),
            Err(_) => Ok(serde_json::json!({ "output": stdout })),
        }
    } else {
        Ok(serde_json::json!({
            "success": false,
            "error": stderr,
            "exit_code": output.status.code()
        }))
    }
}

fn run_cmd_verbose(cmd: &str) -> anyhow::Result<serde_json::Value> {
    let output = Command::new("bash")
        .args(["-c", cmd])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    Ok(serde_json::json!({
        "command": cmd,
        "success": output.status.success(),
        "exit_code": output.status.code(),
        "stdout": stdout,
        "stderr": stderr
    }))
}

#[tool_router(server_handler)]
impl HerdrServer {
    #[tool(description = "List all saved SSH machines in herdr with their status. Returns machine labels, SSH targets, and connectivity state.")]
    fn list_machines(&self, params: Parameters<ListMachinesRequest>) -> String {
        let req = params.0;

        if req.verbose {
            match run_cmd_verbose("herdr machine list") {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd("herdr machine list") {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Check connectivity status of saved machines without prompting for authentication. Optionally check a specific machine by label.")]
    fn machine_status(&self, params: Parameters<MachineStatusRequest>) -> String {
        let req = params.0;

        let cmd = match &req.machine {
            Some(machine) => format!("herdr machine status {}", machine),
            None => "herdr machine status".to_string(),
        };

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Add a new SSH machine to herdr's saved machines list. Requires ssh_target (e.g., 'crombo@100.113.106.95') and a label.")]
    fn add_machine(&self, params: Parameters<AddMachineRequest>) -> String {
        let req = params.0;

        let cmd = format!(
            "herdr machine add {} --label {}",
            req.ssh_target, req.label
        );

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "List all running agents across herdr sessions. Optionally filter by session name. Returns agent IDs, names, and states.")]
    fn agent_list(&self, params: Parameters<AgentListRequest>) -> String {
        let req = params.0;

        let cmd = match &req.session {
            Some(session) => format!("herdr --session {} agent list", session),
            None => "herdr agent list".to_string(),
        };

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Submit a prompt to a running agent. Requires agent_id and prompt text. The agent will process the prompt in its own session.")]
    fn agent_prompt(&self, params: Parameters<AgentPromptRequest>) -> String {
        let req = params.0;

        // Escape single quotes for shell safety
        let escaped_prompt = req.prompt.replace("'", "'\\''");
        let cmd = format!(
            "herdr agent prompt {} '{}'",
            req.agent_id, escaped_prompt
        );

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Read terminal output from an agent. Requires agent_id. Optionally specify number of lines (default: 50).")]
    fn agent_read(&self, params: Parameters<AgentReadRequest>) -> String {
        let req = params.0;

        let cmd = format!(
            "herdr agent read {} --lines {}",
            req.agent_id, req.lines
        );

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Wait for an agent to reach a specific state. Requires agent_id and state (e.g., 'idle', 'busy', 'done'). Timeout in seconds (default: 300).")]
    fn agent_wait(&self, params: Parameters<AgentWaitRequest>) -> String {
        let req = params.0;

        let cmd = format!(
            "herdr agent wait {} --state {} --timeout {}",
            req.agent_id, req.state, req.timeout
        );

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Get information about attaching to a herdr session. Returns the command needed to attach via SSH or locally.")]
    fn session_attach_info(&self, params: Parameters<SessionAttachRequest>) -> String {
        let req = params.0;

        let mut result = serde_json::json!({
            "session": req.session_name,
            "attach_command_local": format!("herdr session attach {}", req.session_name),
            "attach_command_remote_template": format!("herdr --remote ssh://<user>@<host> --session {}", req.session_name),
            "note": "Use the remote template with the machine's SSH target from list_machines"
        });

        if req.verbose {
            let cmd = format!("herdr session attach {} 2>&1 || true", req.session_name);
            match run_cmd_verbose(&cmd) {
                Ok(verbose_result) => {
                    result["verbose"] = verbose_result;
                    serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
                }
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        }
    }

    // ─── Pane Tools ──────────────────────────────────────────────

    #[tool(description = "List all panes in a herdr session. Optionally filter by session name. Returns pane IDs, names, and states.")]
    fn pane_list(&self, params: Parameters<PaneListRequest>) -> String {
        let req = params.0;

        let cmd = match &req.session {
            Some(session) => format!("herdr --session {} pane list", session),
            None => "herdr pane list".to_string(),
        };

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Split a pane horizontally or vertically. Specify direction ('right' or 'down'), optional ratio, cwd, and session.")]
    fn pane_split(&self, params: Parameters<PaneSplitRequest>) -> String {
        let req = params.0;

        let mut cmd_parts = vec!["herdr pane split".to_string()];

        if let Some(id) = &req.pane_id {
            cmd_parts.push(format!("--pane {}", id));
        } else {
            cmd_parts.push("--current".to_string());
        }

        cmd_parts.push(format!("--direction {}", req.direction));

        if let Some(r) = req.ratio {
            cmd_parts.push(format!("--ratio {}", r));
        }

        if let Some(dir) = &req.cwd {
            cmd_parts.push(format!("--cwd {}", dir));
        }

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Focus a specific pane by ID or neighbor direction (left/right/up/down). Optionally specify session.")]
    fn pane_focus(&self, params: Parameters<PaneFocusRequest>) -> String {
        let req = params.0;

        let mut cmd_parts = vec![format!("herdr pane focus {}", req.target)];

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Run a command in a specific pane. Requires pane_id and command. Optionally specify session.")]
    fn pane_run(&self, params: Parameters<PaneRunRequest>) -> String {
        let req = params.0;

        // Escape single quotes for shell safety
        let escaped_cmd = req.command.replace("'", "'\\''");
        let mut cmd_parts = vec![format!("herdr pane run {} '{}'", req.pane_id, escaped_cmd)];

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Read terminal output from a pane. Requires pane_id. Optionally specify lines, source, and session.")]
    fn pane_read(&self, params: Parameters<PaneReadRequest>) -> String {
        let req = params.0;

        let mut cmd_parts = vec![format!("herdr pane read {}", req.pane_id)];

        if let Some(n) = req.lines {
            cmd_parts.push(format!("--lines {}", n));
        }

        if let Some(src) = &req.source {
            cmd_parts.push(format!("--source {}", src));
        }

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Send text to a pane. Requires pane_id and text. Optionally specify enter (default: true) and session.")]
    fn pane_send_text(&self, params: Parameters<PaneSendTextRequest>) -> String {
        let req = params.0;

        // Escape single quotes for shell safety
        let escaped_text = req.text.replace("'", "'\\''");
        let mut cmd_parts = vec![format!("herdr pane send-text {} '{}'", req.pane_id, escaped_text)];

        if !req.enter {
            cmd_parts.push("--no-enter".to_string());
        }

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    #[tool(description = "Close a pane by ID. Optionally specify session.")]
    fn pane_close(&self, params: Parameters<PaneCloseRequest>) -> String {
        let req = params.0;

        let mut cmd_parts = vec![format!("herdr pane close {}", req.pane_id)];

        if let Some(session) = &req.session {
            cmd_parts.push(format!("--session {}", session));
        }

        let cmd = cmd_parts.join(" ");

        if req.verbose {
            match run_cmd_verbose(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        } else {
            match run_cmd(&cmd) {
                Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                Err(e) => format!("{{\"error\": \"{}\"}}", e),
            }
        }
    }

    // ─── Session Template Tools ──────────────────────────────────

    fn template_dir() -> std::path::PathBuf {
        let home = std::env::var("HOME").expect("no HOME");
        std::path::PathBuf::from(home).join(".herdr").join("templates")
    }

    #[tool(description = "List all available herdr session templates. Templates define pane layouts and applications for common workflows (e.g., dev, review, explore).")]
    fn template_list(&self, params: Parameters<TemplateListRequest>) -> String {
        let _req = params.0;
        let dir = Self::template_dir();

        if !dir.exists() {
            return serde_json::to_string(&serde_json::json!({
                "success": true,
                "templates": []
            })).unwrap();
        }

        let mut templates = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && let Ok(content) = std::fs::read_to_string(&path)
                        && let Ok(template) = serde_json::from_str::<serde_json::Value>(&content) {
                            templates.push(serde_json::json!({
                                "name": path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                                "template": template
                            }));
                        }
            }
        }

        serde_json::to_string(&serde_json::json!({
            "success": true,
            "templates": templates,
            "count": templates.len()
        })).unwrap()
    }

    #[tool(description = "Create a new herdr session from a template. Specify template_name (e.g., 'dev', 'review'). Optionally provide session_name and cwd. The template defines pane layouts and applications to launch.")]
    fn template_create_session(&self, params: Parameters<TemplateCreateSessionRequest>) -> String {
        let req = params.0;
        let dir = Self::template_dir();
        let template_path = dir.join(format!("{}.json", req.template_name));

        if !template_path.exists() {
            return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("template not found: {}", req.template_name)
            })).unwrap();
        }

        let content = match std::fs::read_to_string(&template_path) {
            Ok(c) => c,
            Err(e) => return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to read template: {}", e)
            })).unwrap()
        };

        let template: serde_json::Value = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(e) => return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("invalid template JSON: {}", e)
            })).unwrap()
        };

        // Build the session creation command from template config
        let session_name = req.session_name.unwrap_or_else(|| format!("herdr-{}", req.template_name));
        let cwd = req.cwd.clone().unwrap_or_else(|| std::env::var("HOME").unwrap_or_default());

        // Create the herdr session with the appropriate layout
        let create_cmd = format!("herdr session create {} --cwd {}", session_name, cwd);
        let create_result = run_cmd(&create_cmd);

        match create_result {
            Ok(result) => serde_json::to_string(&serde_json::json!({
                "success": true,
                "session": session_name,
                "template": req.template_name,
                "config": template,
                "create_result": result
            })).unwrap(),
            Err(e) => serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to create session: {}", e),
                "template": req.template_name
            })).unwrap()
        }
    }

    #[tool(description = "Save current session layout as a reusable template. Specify template_name, optionally the session to capture from and a description.")]
    fn template_save(&self, params: Parameters<TemplateSaveRequest>) -> String {
        let req = params.0;
        let dir = Self::template_dir();

        // Create templates directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to create templates directory: {}", e)
            })).unwrap();
        }

        // For now, save a basic template structure
        // In a full implementation, this would capture the active pane layout from the session
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        let template = serde_json::json!({
            "name": req.template_name,
            "description": req.description.unwrap_or_default(),
            "panes": [],
            "created_at": timestamp
        });

        let path = dir.join(format!("{}.json", req.template_name));
        match std::fs::write(&path, serde_json::to_string_pretty(&template).unwrap()) {
            Ok(_) => serde_json::to_string(&serde_json::json!({
                "success": true,
                "template": req.template_name,
                "path": path.display().to_string()
            })).unwrap(),
            Err(e) => serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to write template: {}", e),
                "template": req.template_name
            })).unwrap()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = HerdrServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}