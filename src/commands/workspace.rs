use crate::project_loader::ProjectLoader;
use serde_json::json;

/// Handle workspace status
pub async fn handle_workspace_status() -> serde_json::Value {
    let project_root = std::env::current_dir().ok();
    let loader = ProjectLoader::new();

    if let Some(root) = project_root {
        let mut loader = loader;
        if loader.load_from_directory(&root).await.is_ok()
            && let Some(config) = loader.get_current_config()
        {
            return json!({
                "success": true,
                "workspace": {
                    "name": config.name,
                    "description": config.description,
                    "declared_tools": config.tools.len(),
                    "tool_execution_enabled": config.tool_execution_enabled,
                    "user_dinit_socket": config.user_dinit_socket,
                }
            });
        }
    }

    json!({
        "success": true,
        "workspace": null,
    })
}
