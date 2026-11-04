//! Agent management tools: list, prompt, read, wait.

use crate::types::{
    AgentListRequest, AgentPromptRequest, AgentReadRequest, AgentWaitRequest,
};
use rmcp::handler::server::wrapper::Parameters;

pub fn agent_list(params: Parameters<AgentListRequest>) -> String {
    let req = params.0;

    let cmd = match &req.session {
        Some(session) => format!("herdr --session {} agent list", session),
        None => "herdr agent list".to_string(),
    };

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

pub fn agent_prompt(params: Parameters<AgentPromptRequest>) -> String {
    let req = params.0;

    // Escape single quotes for shell safety
    let escaped_prompt = req.prompt.replace("'", "'\\''");
    let cmd = format!(
        "herdr agent prompt {} '{}'",
        req.agent_id, escaped_prompt
    );

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

pub fn agent_read(params: Parameters<AgentReadRequest>) -> String {
    let req = params.0;

    let cmd = format!(
        "herdr agent read {} --lines {}",
        req.agent_id, req.lines
    );

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

pub fn agent_wait(params: Parameters<AgentWaitRequest>) -> String {
    let req = params.0;

    let cmd = format!(
        "herdr agent wait {} --state {} --timeout {}",
        req.agent_id, req.state, req.timeout
    );

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
