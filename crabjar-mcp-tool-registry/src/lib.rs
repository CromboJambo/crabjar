use crabjar_tool_registry::ToolRegistry;
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug)]
pub struct ToolRegistryMcpServer {
    db_path: PathBuf,
    _db_dir: TempDir,
}

impl ToolRegistryMcpServer {
    pub fn new() -> anyhow::Result<Self> {
        let db_dir = tempfile::tempdir()?;
        let db_path = db_dir.path().join("tool_registry.db");

        // Initialize database schema
        let conn = rusqlite::Connection::open(&db_path)?;
        let registry = ToolRegistry::new(&conn);
        registry.init()?;

        Ok(Self {
            db_path,
            _db_dir: db_dir,
        })
    }

    fn get_registry(&self) -> anyhow::Result<ToolRegistry<'static>> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        Ok(ToolRegistry::new(Box::leak(Box::new(conn))))
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListToolsRequest {
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegisterToolRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QueryToolRequest {
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscoverToolsRequest {
    pub project_root: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidateToolsRequest {
    pub tool_names: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AutoRegisterRequest {
    pub tool_names: Vec<String>,
}

#[tool_router(server_handler)]
impl ToolRegistryMcpServer {
    #[tool(description = "List all registered tools in the tool registry")]
    fn list_tools(&self, params: Parameters<ListToolsRequest>) -> String {
        let limit = params.0.limit.unwrap_or(100);
        match self.get_registry() {
            Ok(reg) => match reg.list_all(limit) {
                Ok(tools) => {
                    let values: Vec<Value> = tools
                        .into_iter()
                        .map(|t| serde_json::to_value(t).unwrap())
                        .collect();
                    serde_json::to_string_pretty(&Value::Array(values)).unwrap_or_default()
                }
                Err(e) => format!("Error listing tools: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }

    #[tool(description = "Register a new tool in the registry")]
    fn register_tool(&self, params: Parameters<RegisterToolRequest>) -> String {
        match self.get_registry() {
            Ok(reg) => match reg.register_tool(
                &params.0.name,
                "command",
                &params.0.description.unwrap_or_default(),
                "{}",
                "low_risk",
                3,
                0.9,
                "{}",
            ) {
                Ok(id) => serde_json::to_string_pretty(&serde_json::json!({
                    "id": id,
                    "name": params.0.name
                }))
                .unwrap_or_default(),
                Err(e) => format!("Error registering tool: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }

    #[tool(description = "Query tool details by name")]
    fn query_tool(&self, params: Parameters<QueryToolRequest>) -> String {
        match self.get_registry() {
            Ok(reg) => match reg.query_tool(&params.0.name) {
                Ok(Some(tool)) => {
                    serde_json::to_string_pretty(&serde_json::to_value(tool).unwrap())
                        .unwrap_or_default()
                }
                Ok(None) => format!("Tool '{}' not found", params.0.name),
                Err(e) => format!("Error querying tool: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }

    #[tool(description = "Discover tools from filesystem locations")]
    fn discover_tools(&self, params: Parameters<DiscoverToolsRequest>) -> String {
        let project_root =
            std::path::PathBuf::from(params.0.project_root.unwrap_or_else(|| ".".to_string()));
        match self.get_registry() {
            Ok(reg) => match reg.discover_tools("mcp_server", &project_root) {
                Ok(discovered) => serde_json::to_string_pretty(&serde_json::json!({
                    "count": discovered.len(),
                    "tools": discovered
                }))
                .unwrap_or_default(),
                Err(e) => format!("Error discovering tools: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }

    #[tool(description = "Validate that discovered tools have their required binaries available")]
    fn validate_tools(&self, params: Parameters<ValidateToolsRequest>) -> String {
        match self.get_registry() {
            Ok(reg) => match reg.validate_tools(&params.0.tool_names) {
                Ok(results) => {
                    let values: Vec<Value> = results
                        .iter()
                        .map(|(name, ok, path)| {
                            serde_json::json!({
                                "name": name,
                                "available": ok,
                                "path": path
                            })
                        })
                        .collect();
                    serde_json::to_string_pretty(&serde_json::json!({
                        "results": values
                    }))
                    .unwrap_or_default()
                }
                Err(e) => format!("Error validating tools: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }

    #[tool(description = "Auto-register discovered tools into the registry")]
    fn auto_register(&self, params: Parameters<AutoRegisterRequest>) -> String {
        match self.get_registry() {
            Ok(reg) => match reg.auto_register_discovered(&params.0.tool_names) {
                Ok(registered) => serde_json::to_string_pretty(&serde_json::json!({
                    "registered_count": registered.len(),
                    "ids": registered
                }))
                .unwrap_or_default(),
                Err(e) => format!("Error auto-registering tools: {}", e),
            },
            Err(e) => format!("Error opening registry: {}", e),
        }
    }
}
