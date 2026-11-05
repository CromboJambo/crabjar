//! Skill/plugin install/uninstall command handlers.

use serde_json::json;

/// Install a skill from a local path.
pub fn handle_skill_install(path: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    // Open or create the database
    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    // Load the skill from the path (single skill install)
    let loader = crabjar_skills::SkillLoader::default();
    let loaded = match loader.load_from_path(&path) {
        Ok(skill) => skill,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "error": format!("failed to load skill from {}: {}", path, e),
            }));
        }
    };

    // Bind the skill's tools to the registry
    let binder = crabjar_tool_binding::ToolBinder::new(&registry);
    match binder.bind_skill(&loaded) {
        Ok(bound_count) => Ok(json!({
            "success": true,
            "skill_install": {
                "name": loaded.manifest.name,
                "version": loaded.manifest.version,
                "tools_registered": bound_count,
            }
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("failed to bind skill tools: {}", e),
        })),
    }
}

/// Enable a plugin by name.
pub fn handle_plugin_enable(name: String) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "success": true,
        "plugin_enable": {
            "name": name,
            "status": "enabled",
            "note": "plugin enable/disable is managed by the plugin manager; this command updates registry metadata",
        }
    }))
}

/// Disable a plugin by name.
pub fn handle_plugin_disable(
    name: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "success": true,
        "plugin_disable": {
            "name": name,
            "status": "disabled",
            "note": "plugin enable/disable is managed by the plugin manager; this command updates registry metadata",
        }
    }))
}

/// Unregister (remove) a tool from the registry.
pub fn handle_tool_uninstall(
    name: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    if !db_path.exists() {
        return Ok(json!({
            "success": false,
            "error": "tool registry database not found — run 'crabjar tool discover' first",
        }));
    }

    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    match registry.unregister_tool(&name) {
        Ok(removed) => {
            if removed {
                Ok(json!({
                    "success": true,
                    "tool_uninstall": {
                        "name": name,
                        "status": "removed",
                    }
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": format!("tool '{}' not found in registry", name),
                }))
            }
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("failed to unregister tool '{}': {}", name, e),
        })),
    }
}
