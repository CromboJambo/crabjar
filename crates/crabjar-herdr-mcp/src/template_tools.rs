//! Session template tools: list, create session from template, save layout as template.

use crate::types::{TemplateCreateSessionRequest, TemplateListRequest, TemplateSaveRequest};
use rmcp::handler::server::wrapper::Parameters;
use std::path::PathBuf;

fn template_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("no HOME");
    PathBuf::from(home).join(".herdr").join("templates")
}

pub fn template_list(params: Parameters<TemplateListRequest>) -> String {
    let _req = params.0;
    let dir = template_dir();

    if !dir.exists() {
        return serde_json::to_string(&serde_json::json!({
            "success": true,
            "templates": []
        }))
        .unwrap();
    }

    let mut templates = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(template) = serde_json::from_str::<serde_json::Value>(&content)
            {
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
    }))
    .unwrap()
}

pub fn template_create_session(params: Parameters<TemplateCreateSessionRequest>) -> String {
    let req = params.0;
    let dir = template_dir();
    let template_path = dir.join(format!("{}.json", req.template_name));

    if !template_path.exists() {
        return serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": format!("template not found: {}", req.template_name)
        }))
        .unwrap();
    }

    let content = match std::fs::read_to_string(&template_path) {
        Ok(c) => c,
        Err(e) => {
            return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to read template: {}", e)
            }))
            .unwrap();
        }
    };

    let template: serde_json::Value = match serde_json::from_str(&content) {
        Ok(t) => t,
        Err(e) => {
            return serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("invalid template JSON: {}", e)
            }))
            .unwrap();
        }
    };

    // Build the session creation command from template config
    let session_name = req
        .session_name
        .unwrap_or_else(|| format!("herdr-{}", req.template_name));
    let cwd = req
        .cwd
        .clone()
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_default());

    // Create the herdr session with the appropriate layout
    let create_cmd = format!("herdr session create {} --cwd {}", session_name, cwd);
    let create_result = crate::run_cmd(&create_cmd);

    match create_result {
        Ok(result) => serde_json::to_string(&serde_json::json!({
            "success": true,
            "session": session_name,
            "template": req.template_name,
            "config": template,
            "create_result": result
        }))
        .unwrap(),
        Err(e) => serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": format!("failed to create session: {}", e),
            "template": req.template_name
        }))
        .unwrap(),
    }
}

pub fn template_save(params: Parameters<TemplateSaveRequest>) -> String {
    let req = params.0;
    let dir = template_dir();

    // Create templates directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": format!("failed to create templates directory: {}", e)
        }))
        .unwrap();
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
        }))
        .unwrap(),
        Err(e) => serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": format!("failed to write template: {}", e),
            "template": req.template_name
        }))
        .unwrap(),
    }
}
