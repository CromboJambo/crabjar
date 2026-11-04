//! Session management tools: attach info.

use crate::types::SessionAttachRequest;
use rmcp::handler::server::wrapper::Parameters;

pub fn session_attach_info(params: Parameters<SessionAttachRequest>) -> String {
    let req = params.0;

    let mut result = serde_json::json!({
        "session": req.session_name,
        "attach_command_local": format!("herdr session attach {}", req.session_name),
        "attach_command_remote_template": format!("herdr --remote ssh://<user>@<host> --session {}", req.session_name),
        "note": "Use the remote template with the machine's SSH target from list_machines"
    });

    if req.verbose {
        let cmd = format!("herdr session attach {} 2>&1 || true", req.session_name);
        match crate::run_cmd_verbose(&cmd) {
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
