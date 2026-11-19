use crabjar_lib::BackendCommand;
use serde_json::json;

/// Handle backend commands
pub fn handle_backend_command(command: BackendCommand) -> Result<serde_json::Value, String> {
    match command {
        BackendCommand::Set { backend } => {
            // Validate backend type
            match backend.as_str() {
                "unsloth-studio" | "native" => {
                    // Update environment variable for the orchestrator
                    unsafe {
                        std::env::set_var("INFERENCE_BACKEND", &backend);
                    }
                    Ok(json!({
                        "success": true,
                        "message": format!("Inference backend set to: {}", backend),
                    }))
                }
                _ => Err(format!(
                    "Invalid backend: {}. Use 'unsloth-studio' or 'native'.",
                    backend
                )),
            }
        }
        BackendCommand::Get => {
            let current_backend =
                std::env::var("INFERENCE_BACKEND").unwrap_or_else(|_| "unsloth-studio".to_string());
            Ok(json!({
                "success": true,
                "current_backend": current_backend,
            }))
        }
    }
}
