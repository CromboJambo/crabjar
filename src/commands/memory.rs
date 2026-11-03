use crabjar_lib::MemoryCommand;
use serde_json::json;

fn error_response(message: &str) -> serde_json::Value {
    json!({
        "success": false,
        "error": message,
    })
}

/// Handle memory commands (P2-1 Hermes compatibility layer)
pub fn handle_memory_command(command: MemoryCommand) -> serde_json::Value {
    // Placeholder implementation — actual MemoryStore integration would go here
    match command {
        MemoryCommand::Add { target, content } => json!({
            "success": true,
            "message": format!("memory entry added to {}", target),
            "target": target,
            "content_length": content.len()
        }),
        MemoryCommand::Patch { target, old_text, new_text } => json!({
            "success": true,
            "message": format!("memory entry patched in {}", target),
            "target": target,
            "old_length": old_text.len(),
            "new_length": new_text.len()
        }),
        MemoryCommand::Remove { target, old_text } => json!({
            "success": true,
            "message": format!("memory entry removed from {}", target),
            "target": target,
            "removed_length": old_text.len()
        }),
    }
}
