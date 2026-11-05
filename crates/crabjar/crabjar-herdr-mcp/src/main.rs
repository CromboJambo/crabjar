//! crabjar-herdr-mcp: MCP server wrapping herdr for fleet machine management and agent delegation.
//!
//! Exposes tools for listing machines, checking status, adding machines, managing agents,
//! delegating tasks across the home-lab fleet via herdr's SSH integration, and controlling
//! panes within herdr sessions.

mod agent_tools;
mod machine_tools;
mod pane_tools;
mod session_tools;
mod template_tools;
mod types;

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::process::Command;

fn run_cmd(cmd: &str) -> anyhow::Result<serde_json::Value> {
    let output = Command::new("bash").args(["-c", cmd]).output()?;
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
    let output = Command::new("bash").args(["-c", cmd]).output()?;
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

#[derive(Debug, Clone)]
pub struct HerdrServer;

#[tool_router(server_handler)]
impl HerdrServer {
    #[tool(description = "List all saved SSH machines in herdr with their status.")]
    fn list_machines(&self, params: Parameters<types::ListMachinesRequest>) -> String {
        machine_tools::list_machines(params)
    }

    #[tool(description = "Check connectivity status of saved machines.")]
    fn machine_status(&self, params: Parameters<types::MachineStatusRequest>) -> String {
        machine_tools::machine_status(params)
    }

    #[tool(description = "Add a new SSH machine to herdr's saved machines list.")]
    fn add_machine(&self, params: Parameters<types::AddMachineRequest>) -> String {
        machine_tools::add_machine(params)
    }

    #[tool(description = "List all running agents across herdr sessions.")]
    fn agent_list(&self, params: Parameters<types::AgentListRequest>) -> String {
        agent_tools::agent_list(params)
    }

    #[tool(description = "Submit a prompt to a running agent.")]
    fn agent_prompt(&self, params: Parameters<types::AgentPromptRequest>) -> String {
        agent_tools::agent_prompt(params)
    }

    #[tool(description = "Read terminal output from an agent.")]
    fn agent_read(&self, params: Parameters<types::AgentReadRequest>) -> String {
        agent_tools::agent_read(params)
    }

    #[tool(description = "Wait for an agent to reach a specific state.")]
    fn agent_wait(&self, params: Parameters<types::AgentWaitRequest>) -> String {
        agent_tools::agent_wait(params)
    }

    #[tool(description = "Get information about attaching to a herdr session.")]
    fn session_attach_info(&self, params: Parameters<types::SessionAttachRequest>) -> String {
        session_tools::session_attach_info(params)
    }

    #[tool(description = "List all panes in a herdr session.")]
    fn pane_list(&self, params: Parameters<types::PaneListRequest>) -> String {
        pane_tools::pane_list(params)
    }

    #[tool(description = "Split a pane horizontally or vertically.")]
    fn pane_split(&self, params: Parameters<types::PaneSplitRequest>) -> String {
        pane_tools::pane_split(params)
    }

    #[tool(description = "Focus a specific pane by ID or neighbor direction.")]
    fn pane_focus(&self, params: Parameters<types::PaneFocusRequest>) -> String {
        pane_tools::pane_focus(params)
    }

    #[tool(description = "Run a command in a specific pane.")]
    fn pane_run(&self, params: Parameters<types::PaneRunRequest>) -> String {
        pane_tools::pane_run(params)
    }

    #[tool(description = "Read terminal output from a pane.")]
    fn pane_read(&self, params: Parameters<types::PaneReadRequest>) -> String {
        pane_tools::pane_read(params)
    }

    #[tool(description = "Send text to a pane.")]
    fn pane_send_text(&self, params: Parameters<types::PaneSendTextRequest>) -> String {
        pane_tools::pane_send_text(params)
    }

    #[tool(description = "Close a pane by ID.")]
    fn pane_close(&self, params: Parameters<types::PaneCloseRequest>) -> String {
        pane_tools::pane_close(params)
    }

    #[tool(description = "List all available herdr session templates.")]
    fn template_list(&self, params: Parameters<types::TemplateListRequest>) -> String {
        template_tools::template_list(params)
    }

    #[tool(description = "Create a new herdr session from a template.")]
    fn template_create_session(
        &self,
        params: Parameters<types::TemplateCreateSessionRequest>,
    ) -> String {
        template_tools::template_create_session(params)
    }

    #[tool(description = "Save current session layout as a reusable template.")]
    fn template_save(&self, params: Parameters<types::TemplateSaveRequest>) -> String {
        template_tools::template_save(params)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = HerdrServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
