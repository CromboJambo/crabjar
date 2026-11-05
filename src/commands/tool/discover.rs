//! Tool discovery command handler.

use serde_json::json;

/// Discover tools from known sources and auto-register them.
pub async fn handle_tool_discover(
    source: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    // Open or create the database
    let conn = rusqlite::Connection::open(&db_path)?;
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let registry = crabjar_tool_registry::ToolRegistry::new(&conn);

    // Discover tools
    let discovered = registry.discover_tools(&source, &project_root)?;

    // Validate tool availability
    let validation = registry.validate_tools(&discovered)?;

    // Auto-register discovered tools
    let registered = registry.auto_register_discovered(&discovered)?;

    // Build response with availability info
    let tool_info: Vec<serde_json::Value> = validation
        .iter()
        .map(|(name, available, path)| {
            json!({
                "name": name,
                "available": available,
                "path": path,
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "discovery": {
            "source": source,
            "discovered": discovered.len(),
            "registered": registered.len(),
            "tools": tool_info,
            "warning": if discovered.is_empty() {
                "no tools discovered — ensure skills have manifest.json files or MCP configs exist"
            } else {
                ""
            },
        }
    }))
}
