//! Tool list command handler.

use serde_json::json;

/// Mapped tool row from a SQLite query.
type ToolRow = (String, String, String, String, u32, f64, String);

fn map_tool_row(row: &rusqlite::Row) -> Result<ToolRow, rusqlite::Error> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

/// List registered tools from the tool registry database.
pub fn handle_tool_list(
    r#type: Option<String>,
    limit: usize,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let db_path = project_root.join("tool_registry/tool_registry.db");

    if !db_path.exists() {
        return Ok(json!({
            "success": true,
            "tools": {
                "tools": [],
                "count": 0,
                "warning": "tool registry database not found — run 'crabjar tool discover' to populate"
            }
        }));
    }

    let conn = rusqlite::Connection::open(&db_path)?;

    // Initialize schema if needed
    let schema = crabjar_tool_registry::schema::TOOL_REGISTRY_SCHEMA;
    conn.execute_batch(schema)?;

    let tools: Vec<serde_json::Value> = {
        let query = if let Some(ref _tool_type) = r#type {
            "SELECT id, name, type, description, trust_layer, confidence, execution_policy \
                 FROM tools WHERE type = ?1 ORDER BY last_used DESC LIMIT ?2"
                .to_string()
        } else {
            "SELECT id, name, type, description, trust_layer, confidence, execution_policy \
                 FROM tools ORDER BY last_used DESC LIMIT ?1"
                .to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let rows = if let Some(ref tool_type) = r#type {
            stmt.query_map(rusqlite::params![tool_type, limit as i64], map_tool_row)?
        } else {
            stmt.query_map(rusqlite::params![limit as i64], map_tool_row)?
        };

        rows.filter_map(|row| row.ok())
            .map(
                |(id, name, tool_type, description, trust_layer, confidence, policy)| {
                    json!({
                        "id": id,
                        "name": name,
                        "type": tool_type,
                        "description": description,
                        "trust_layer": trust_layer,
                        "confidence": confidence,
                        "execution_policy": policy,
                    })
                },
            )
            .collect()
    };

    Ok(json!({
        "success": true,
        "tools": {
            "tools": tools,
            "count": tools.len(),
            "type_filter": r#type,
            "limit": limit,
        }
    }))
}
