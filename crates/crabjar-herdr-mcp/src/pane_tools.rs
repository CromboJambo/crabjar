//! Pane management tools: list, split, focus, run, read, send-text, close.

use crate::types::{
    PaneCloseRequest, PaneFocusRequest, PaneListRequest, PaneReadRequest,
    PaneRunRequest, PaneSendTextRequest, PaneSplitRequest,
};
use rmcp::handler::server::wrapper::Parameters;

pub fn pane_list(params: Parameters<PaneListRequest>) -> String {
    let req = params.0;

    let cmd = match &req.session {
        Some(session) => format!("herdr --session {} pane list", session),
        None => "herdr pane list".to_string(),
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

pub fn pane_split(params: Parameters<PaneSplitRequest>) -> String {
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

pub fn pane_focus(params: Parameters<PaneFocusRequest>) -> String {
    let req = params.0;

    let mut cmd_parts = vec![format!("herdr pane focus {}", req.target)];

    if let Some(session) = &req.session {
        cmd_parts.push(format!("--session {}", session));
    }

    let cmd = cmd_parts.join(" ");

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

pub fn pane_run(params: Parameters<PaneRunRequest>) -> String {
    let req = params.0;

    // Escape single quotes for shell safety
    let escaped_cmd = req.command.replace("'", "'\\''");
    let mut cmd_parts = vec![format!("herdr pane run {} '{}'", req.pane_id, escaped_cmd)];

    if let Some(session) = &req.session {
        cmd_parts.push(format!("--session {}", session));
    }

    let cmd = cmd_parts.join(" ");

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

pub fn pane_read(params: Parameters<PaneReadRequest>) -> String {
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

pub fn pane_send_text(params: Parameters<PaneSendTextRequest>) -> String {
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

pub fn pane_close(params: Parameters<PaneCloseRequest>) -> String {
    let req = params.0;

    let mut cmd_parts = vec![format!("herdr pane close {}", req.pane_id)];

    if let Some(session) = &req.session {
        cmd_parts.push(format!("--session {}", session));
    }

    let cmd = cmd_parts.join(" ");

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
