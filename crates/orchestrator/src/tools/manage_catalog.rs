//! manage_catalog tool — tool catalog management (P2 parity).
//!
//! Wraps the crabjar-tool-registry crate to expose catalog operations
//! as a first-class orchestrator tool. Supports list, register, query,
//! discover, validate, and auto_register actions.

use crabjar_tool_registry::ToolRegistry;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tempfile::TempDir;

static REGISTRY: OnceLock<Arc<RegistryHandle>> = OnceLock::new();

struct RegistryHandle {
    db_path: PathBuf,
    _db_dir: TempDir,
}

impl RegistryHandle {
    fn new() -> anyhow::Result<Self> {
        let db_dir = tempfile::tempdir()?;
        let db_path = db_dir.path().join("tool_catalog.db");

        // Initialize database schema
        let conn = Connection::open(&db_path)?;
        let registry = ToolRegistry::new(&conn);
        registry.init()?;

        Ok(Self {
            db_path,
            _db_dir: db_dir,
        })
    }

    fn get_registry(&self) -> anyhow::Result<ToolRegistry<'static>> {
        let conn = Connection::open(&self.db_path)?;
        Ok(ToolRegistry::new(Box::leak(Box::new(conn))))
    }
}

fn get_registry_handle() -> Result<&'static RegistryHandle, String> {
    if let Some(handle) = REGISTRY.get() {
        return Ok(handle.as_ref());
    }
    let handle = RegistryHandle::new().map_err(|e| e.to_string())?;
    let arc = Arc::new(handle);
    REGISTRY.get_or_init(|| arc.clone());
    Ok(REGISTRY.get().unwrap())
}

/// Handle manage_catalog tool invocation.
///
/// Arguments:
/// - args[0]: JSON string with fields:
///   - `action` (required): "list" | "register" | "query" | "discover" | "validate" | "auto_register"
///   - Action-specific fields follow
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("manage_catalog requires a JSON argument with 'action' field".to_string());
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Failed to parse manage_catalog arguments: {}", e))?;

    let action = request["action"]
        .as_str()
        .ok_or("manage_catalog requires 'action' field")?;

    match action {
        "list" => handle_list(&request),
        "register" => handle_register(&request),
        "query" => handle_query(&request),
        "discover" => handle_discover(&request),
        "validate" => handle_validate(&request),
        "auto_register" => handle_auto_register(&request),
        _ => Err(format!("Unknown manage_catalog action: {}", action)),
    }
}

fn handle_list(request: &Value) -> Result<String, String> {
    let limit = request["limit"].as_u64().unwrap_or(100) as usize;
    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.list_all(limit) {
        Ok(tools) => {
            let values: Vec<Value> = tools
                .into_iter()
                .map(|t| serde_json::to_value(t).unwrap_or(json!({"error": "serialization failed"})))
                .collect();
            Ok(serde_json::to_string_pretty(&json!({
                "count": values.len(),
                "tools": values
            })).map_err(|e| e.to_string())?)
        }
        Err(e) => Err(format!("Error listing tools: {}", e)),
    }
}

fn handle_register(request: &Value) -> Result<String, String> {
    let name = request["name"]
        .as_str()
        .ok_or("register action requires 'name' field")?;
    let description = request["description"].as_str().unwrap_or("");
    let tool_type = request["type"].as_str().unwrap_or("command");
    let risk_level = request["risk_level"].as_str().unwrap_or("low_risk");
    let priority = request["priority"].as_u64().unwrap_or(3) as u32;
    let reliability = request["reliability"].as_f64().unwrap_or(0.9);

    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.register_tool(name, tool_type, description, "{}", risk_level, priority, reliability, "{}") {
        Ok(id) => Ok(serde_json::to_string_pretty(&json!({
            "status": "registered",
            "id": id,
            "name": name
        })).map_err(|e| e.to_string())?),
        Err(e) => Err(format!("Error registering tool: {}", e)),
    }
}

fn handle_query(request: &Value) -> Result<String, String> {
    let name = request["name"]
        .as_str()
        .ok_or("query action requires 'name' field")?;

    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.query_tool(name) {
        Ok(Some(tool)) => serde_json::to_string_pretty(&serde_json::to_value(tool).unwrap_or(json!({"error": "serialization failed"}))).map_err(|e| e.to_string()),
        Ok(None) => Err(format!("Tool '{}' not found in catalog", name)),
        Err(e) => Err(format!("Error querying tool: {}", e)),
    }
}

fn handle_discover(request: &Value) -> Result<String, String> {
    let project_root = request["project_root"]
        .as_str()
        .unwrap_or(".");
    let source_type = request["source_type"].as_str().unwrap_or("mcp_server");

    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.discover_tools(source_type, &PathBuf::from(project_root)) {
        Ok(discovered) => Ok(serde_json::to_string_pretty(&json!({
            "count": discovered.len(),
            "tools": discovered
        })).map_err(|e| e.to_string())?),
        Err(e) => Err(format!("Error discovering tools: {}", e)),
    }
}

fn handle_validate(request: &Value) -> Result<String, String> {
    let tool_names: Vec<String> = request["tool_names"]
        .as_array()
        .ok_or("validate action requires 'tool_names' array")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if tool_names.is_empty() {
        return Err("validate action requires at least one tool name".to_string());
    }

    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.validate_tools(&tool_names) {
        Ok(results) => {
            let values: Vec<Value> = results
                .iter()
                .map(|(name, available, path)| json!({
                    "name": name,
                    "available": available,
                    "path": path
                }))
                .collect();
            Ok(serde_json::to_string_pretty(&json!({
                "results": values
            })).map_err(|e| e.to_string())?)
        }
        Err(e) => Err(format!("Error validating tools: {}", e)),
    }
}

fn handle_auto_register(request: &Value) -> Result<String, String> {
    let tool_names: Vec<String> = request["tool_names"]
        .as_array()
        .ok_or("auto_register action requires 'tool_names' array")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if tool_names.is_empty() {
        return Err("auto_register action requires at least one tool name".to_string());
    }

    let handle = get_registry_handle()?;
    let reg = handle.get_registry().map_err(|e| e.to_string())?;

    match reg.auto_register_discovered(&tool_names) {
        Ok(registered) => Ok(serde_json::to_string_pretty(&json!({
            "status": "auto_registered",
            "count": registered.len(),
            "ids": registered
        })).map_err(|e| e.to_string())?),
        Err(e) => Err(format!("Error auto-registering tools: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manage_catalog_missing_action() {
        let args = vec![json!({}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'action'"));
    }

    #[tokio::test]
    async fn test_manage_catalog_unknown_action() {
        let args = vec![json!({"action": "nonexistent"}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown manage_catalog action"));
    }

    #[tokio::test]
    async fn test_manage_catalog_list() {
        let args = vec![json!({"action": "list"}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_ok());
        let parsed: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(parsed["count"].is_u64());
    }

    #[tokio::test]
    async fn test_manage_catalog_register_and_query() {
        // Register a tool
        let register_args = vec![json!({
            "action": "register",
            "name": "test_tool_parity_check",
            "description": "Test tool for parity verification"
        }).to_string()];
        let result = handle(&register_args).await;
        assert!(result.is_ok());

        // Query it back
        let query_args = vec![json!({
            "action": "query",
            "name": "test_tool_parity_check"
        }).to_string()];
        let result = handle(&query_args).await;
        assert!(result.is_ok());
        let parsed: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed["name"], "test_tool_parity_check");
    }

    #[tokio::test]
    async fn test_manage_catalog_register_missing_name() {
        let args = vec![json!({"action": "register"}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'name'"));
    }

    #[tokio::test]
    async fn test_manage_catalog_validate_empty() {
        let args = vec![json!({"action": "validate", "tool_names": []}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one tool name"));
    }
}
