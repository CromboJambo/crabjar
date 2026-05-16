use crate::error::ToolRegistryError;
use crate::schema::{
    init_db, list_all_tools, list_tools_by_type, query_discovery, query_tool, query_tool_usage,
    record_tool_discovery, record_tool_usage, register_tool,
};
use rusqlite::Connection;
use tracing::debug;

/// MCP tool registry with rig/mistral.rs patterns for tool discovery, registration, and execution policy.
///
/// Core primitives: Agent (LLM with preamble, static/dynamic context, tools), EmbeddingsBuilder, Extractor, VectorStoreIndex.
/// Vector store integrations: LanceDB, MongoDB, Qdrant, PostgreSQL, SQLite, SurrealDB, Milvus, ScyllaDB, Neo4j, S3Vectors, HelixDB, Vectorize.
pub struct ToolRegistry<'a> {
    conn: &'a Connection,
}

impl<'a> ToolRegistry<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the tool registry database.
    pub fn init(&self) -> Result<(), ToolRegistryError> {
        init_db(self.conn).map_err(ToolRegistryError::Schema)
    }

    /// Register a tool.
    #[allow(clippy::too_many_arguments)]
    pub fn register_tool(
        &self,
        name: &str,
        r#type: &str,
        description: &str,
        schema: &str,
        execution_policy: &str,
        trust_layer: u32,
        confidence: f64,
        metadata: &str,
    ) -> Result<String, ToolRegistryError> {
        register_tool(
            self.conn,
            name,
            r#type,
            description,
            schema,
            execution_policy,
            trust_layer,
            confidence,
            metadata,
        )
        .map_err(ToolRegistryError::Schema)
    }

    /// Query a tool by name.
    pub fn query_tool(
        &self,
        name: &str,
    ) -> Result<Option<crate::schema::ToolRow>, ToolRegistryError> {
        query_tool(self.conn, name).map_err(ToolRegistryError::Schema)
    }

    /// List all tools.
    pub fn list_all(&self, limit: usize) -> Result<Vec<crate::schema::ToolRow>, ToolRegistryError> {
        list_all_tools(self.conn, limit).map_err(ToolRegistryError::Schema)
    }

    /// List tools by type.
    pub fn list_by_type(
        &self,
        r#type: &str,
        limit: usize,
    ) -> Result<Vec<crate::schema::ToolRow>, ToolRegistryError> {
        list_tools_by_type(self.conn, r#type, limit).map_err(ToolRegistryError::Schema)
    }

    /// Record tool usage metrics.
    pub fn record_usage(
        &self,
        tool_id: &str,
        session_id: &str,
        call_count: i32,
        success_rate: f64,
        avg_latency: f64,
    ) -> Result<(), ToolRegistryError> {
        record_tool_usage(
            self.conn,
            tool_id,
            session_id,
            call_count,
            success_rate,
            avg_latency,
        )
        .map_err(ToolRegistryError::Schema)
    }

    /// Query tool usage metrics.
    pub fn query_usage(
        &self,
        tool_id: &str,
    ) -> Result<Vec<crate::schema::UsageRow>, ToolRegistryError> {
        query_tool_usage(self.conn, tool_id).map_err(ToolRegistryError::Schema)
    }

    /// Record tool discovery.
    pub fn record_discovery(
        &self,
        source: &str,
        tool_name: &str,
    ) -> Result<String, ToolRegistryError> {
        record_tool_discovery(self.conn, source, tool_name).map_err(ToolRegistryError::Schema)
    }

    /// Query tool discovery history.
    pub fn query_discovery(
        &self,
        source: &str,
        limit: usize,
    ) -> Result<Vec<crate::schema::DiscoveryRow>, ToolRegistryError> {
        query_discovery(self.conn, source, limit).map_err(ToolRegistryError::Schema)
    }

    /// Generate tool execution configuration.
    pub fn generate_execution_config(
        &self,
        tool_name: &str,
        trust_layer: u32,
        confidence: f64,
    ) -> Result<String, ToolRegistryError> {
        let mut config = String::new();
        config.push_str(&format!("tool = {}\n", tool_name));
        config.push_str(&format!("trust_layer = {}\n", trust_layer));
        config.push_str(&format!("confidence = {}\n", confidence));
        config.push_str(&format!(
            "auto_execute = {}\n",
            if trust_layer >= 3 && confidence >= 0.8 {
                "true"
            } else {
                "false"
            }
        ));

        debug!(
            tool_name = %tool_name,
            trust_layer = trust_layer,
            confidence = confidence,
            "Tool registry: execution config generated"
        );

        Ok(config)
    }

    /// Discover tools from a source by scanning skill directories and MCP manifests.
    pub async fn discover_tools(
        &self,
        source: &str,
        project_root: &std::path::Path,
    ) -> Result<Vec<String>, ToolRegistryError> {
        let mut discovered = Vec::new();

        // Scan project-level skill directories for tool definitions
        for ancestor in project_root.ancestors() {
            let candidate = ancestor.join(".agents/skills");
            if candidate.is_dir() {
                for entry in std::fs::read_dir(&candidate)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_md = path.join("SKILL.md");
                        if skill_md.exists() {
                            let content = std::fs::read_to_string(&skill_md)?;
                            for line in content.lines() {
                                if line.contains("tool")
                                    || line.contains("function")
                                    || line.contains("command")
                                {
                                    let tool_name = line
                                        .split_once(':')
                                        .or_else(|| line.split_once('='))
                                        .map(|(k, _v)| k.trim())
                                        .unwrap_or(line.trim());
                                    if !tool_name.is_empty()
                                        && !discovered.contains(&tool_name.to_string())
                                    {
                                        discovered.push(tool_name.to_string());
                                        self.record_discovery(source, tool_name)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Scan user-level skill directories
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        for scope in [".corust-agent/skills", ".agents/skills"] {
            let candidate = std::path::Path::new(&home_dir).join(scope);
            if candidate.is_dir() {
                for entry in std::fs::read_dir(&candidate)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() && path.join("SKILL.md").exists() {
                        let content = std::fs::read_to_string(path.join("SKILL.md"))?;
                        for line in content.lines() {
                            if line.contains("tool")
                                || line.contains("function")
                                || line.contains("command")
                            {
                                let tool_name = line
                                    .split_once(':')
                                    .or_else(|| line.split_once('='))
                                    .map(|(k, _v)| k.trim())
                                    .unwrap_or(line.trim());
                                if !tool_name.is_empty()
                                    && !discovered.contains(&tool_name.to_string())
                                {
                                    discovered.push(tool_name.to_string());
                                    self.record_discovery(source, tool_name)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!(
            source = %source,
            tool_count = discovered.len(),
            "Tool registry: tools discovered"
        );

        Ok(discovered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tool_registry_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let rows = registry.list_all(10).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_register_and_query_tool() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let id = registry
            .register_tool(
                "cargo_check",
                "command",
                "Run cargo check on workspace",
                "{\"tool\": \"cargo\", \"args\": [\"check\", \"--workspace\"]}",
                "low_risk",
                3,
                0.9,
                "{}",
            )
            .unwrap();

        let row = registry.query_tool("cargo_check").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().name, "cargo_check");
    }

    #[test]
    fn test_list_all_tools() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        registry
            .register_tool(
                "cargo_check",
                "command",
                "Run cargo check on workspace",
                "{\"tool\": \"cargo\", \"args\": [\"check\", \"--workspace\"]}",
                "low_risk",
                3,
                0.9,
                "{}",
            )
            .unwrap();

        registry
            .register_tool(
                "openai_chat",
                "llm",
                "OpenAI chat completion",
                "{\"provider\": \"openai\", \"model\": \"gpt-4o\"}",
                "medium_risk",
                2,
                0.7,
                "{}",
            )
            .unwrap();

        let rows = registry.list_all(10).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_record_usage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let id = registry
            .register_tool(
                "cargo_check",
                "command",
                "Run cargo check on workspace",
                "{\"tool\": \"cargo\", \"args\": [\"check\", \"--workspace\"]}",
                "low_risk",
                3,
                0.9,
                "{}",
            )
            .unwrap();

        registry
            .record_usage(&id, "session-1", 5, 1.0, 0.5)
            .unwrap();

        let rows = registry.query_usage(&id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].call_count, 5);
    }

    #[test]
    fn test_generate_execution_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let config = registry
            .generate_execution_config("cargo_check", 3, 0.9)
            .unwrap();

        assert!(config.contains("tool = cargo_check"));
        assert!(config.contains("auto_execute = true"));
    }
}
