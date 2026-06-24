use crate::error::ToolRegistryError;
use crate::schema::{
    ToolRegistrySchemaError, init_db, list_all_tools, list_tools_by_type, query_discovery,
    query_tool, query_tool_usage, record_tool_discovery, record_tool_usage, register_tool,
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

    /// Discover tools from a source by scanning skill directories, MCP manifests, and state-docs.
    ///
    /// Scans three layers:
    /// 1. Project-level skill directories (`.agents/skills/*/manifest.json`)
    /// 2. User-level skill directories (`~/.corust-agent/skills`, `~/.agents/skills`)
    /// 3. MCP server configurations (`~/.config/mcp/`, `~/.config/crabjar/mcp/`)
    /// 4. State-docs registered tools
    pub async fn discover_tools(
        &self,
        source: &str,
        project_root: &std::path::Path,
    ) -> Result<Vec<String>, ToolRegistryError> {
        let mut discovered = Vec::new();

        // Layer 1: Scan project-level skill directories for tool definitions
        for ancestor in project_root.ancestors() {
            let candidate = ancestor.join(".agents/skills");
            if candidate.is_dir() {
                for entry in std::fs::read_dir(&candidate)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest = path.join("manifest.json");
                        if manifest.exists() {
                            let content = std::fs::read_to_string(&manifest)?;
                            let parsed: serde_json::Value = serde_json::from_str(&content)
                                .map_err(|e| {
                                    ToolRegistryError::Schema(ToolRegistrySchemaError::SchemaError(
                                        e.to_string(),
                                    ))
                                })?;
                            if let Some(tools) = parsed["tools"].as_array() {
                                for tool in tools {
                                    if let Some(name) = tool["name"].as_str()
                                        && !discovered.contains(&name.to_string())
                                    {
                                        discovered.push(name.to_string());
                                        self.record_discovery(source, name)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Layer 2: Scan user-level skill directories
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        for scope in [".corust-agent/skills", ".agents/skills"] {
            let candidate = std::path::Path::new(&home_dir).join(scope);
            if candidate.is_dir() {
                for entry in std::fs::read_dir(&candidate)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() && path.join("manifest.json").exists() {
                        let content = std::fs::read_to_string(path.join("manifest.json"))?;
                        let parsed: serde_json::Value =
                            serde_json::from_str(&content).map_err(|e| {
                                ToolRegistryError::Schema(ToolRegistrySchemaError::SchemaError(
                                    e.to_string(),
                                ))
                            })?;
                        if let Some(tools) = parsed["tools"].as_array() {
                            for tool in tools {
                                if let Some(name) = tool["name"].as_str()
                                    && !discovered.contains(&name.to_string())
                                {
                                    discovered.push(name.to_string());
                                    self.record_discovery(source, name)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Layer 3: Scan MCP server configurations
        for mcp_dir in [
            std::path::Path::new(&home_dir).join(".config/mcp"),
            std::path::Path::new(&home_dir).join(".config/crabjar/mcp"),
        ] {
            if mcp_dir.is_dir() {
                for entry in std::fs::read_dir(&mcp_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                        let content = std::fs::read_to_string(&path)?;
                        let parsed: serde_json::Value =
                            serde_json::from_str(&content).map_err(|e| {
                                ToolRegistryError::Schema(ToolRegistrySchemaError::SchemaError(
                                    e.to_string(),
                                ))
                            })?;
                        // MCP config may have a "tools" array or "command"/"args" fields
                        if let Some(tools) = parsed["tools"].as_array() {
                            for tool in tools {
                                if let Some(name) = tool["name"].as_str()
                                    && !discovered.contains(&name.to_string())
                                {
                                    discovered.push(name.to_string());
                                    self.record_discovery(source, name)?;
                                }
                            }
                        } else if let Some(cmd) = parsed["command"].as_str()
                            && let Some(args) = parsed["args"].as_array()
                        {
                            let tool_name = format!(
                                "{}-{}",
                                cmd.rsplit('/').next().unwrap_or("mcp"),
                                args.first().and_then(|a| a.as_str()).unwrap_or("server")
                            );
                            if !discovered.contains(&tool_name) {
                                let name_clone = tool_name.clone();
                                discovered.push(tool_name);
                                self.record_discovery(source, &name_clone)?;
                            }
                        }
                    }
                }
            }
        }

        // Layer 4: Discover tools from state-docs annotations
        let state_docs_dir = project_root.join("state-docs");
        if state_docs_dir.is_dir() {
            for entry in std::fs::read_dir(&state_docs_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file()
                    && path.extension().is_some_and(|ext| ext == "md")
                    && path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("tool_"))
                    && let Some(name) = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.trim_start_matches("tool_").to_string())
                        .filter(|n| !n.is_empty())
                    && !discovered.contains(&name)
                {
                    discovered.push(name.clone());
                    self.record_discovery(source, &name)?;
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

    /// Validate that discovered tools have their required binaries available.
    ///
    /// Returns a map of tool names to their validation status.
    pub fn validate_tools(
        &self,
        tool_names: &[String],
    ) -> Result<Vec<(String, bool, Option<String>)>, ToolRegistryError> {
        let mut results = Vec::new();

        for name in tool_names {
            // Check if the tool name matches a known binary
            let binary_name = match name.as_str() {
                "cargo_check" | "cargo" => "cargo",
                "git_diff" | "git" | "git_status" => "git",
                "lint" | "clippy" => "cargo", // clippy is a cargo subcommand
                "rustfmt" => "rustfmt",
                "bitwarden" | "bw" => "bw",
                "docker" => "docker",
                "kubectl" => "kubectl",
                "npm" => "npm",
                "yarn" => "yarn",
                "python" | "pip" | "pip3" => "python3",
                _ => name.as_str(),
            };

            match which::which(binary_name) {
                Ok(path) => {
                    results.push((name.clone(), true, Some(path.to_string_lossy().to_string())))
                }
                Err(_) => results.push((name.clone(), false, None)),
            }
        }

        Ok(results)
    }

    /// Auto-register discovered tools into the registry.
    ///
    /// For each discovered tool, if it doesn't already exist in the registry,
    /// register it with a default low-risk execution policy.
    pub fn auto_register_discovered(
        &self,
        tool_names: &[String],
    ) -> Result<Vec<String>, ToolRegistryError> {
        let mut registered = Vec::new();

        for name in tool_names {
            // Check if tool already exists
            if self.query_tool(name)?.is_some() {
                continue;
            }

            // Register with defaults
            let id = self.register_tool(
                name,
                "mcp",
                &format!("Discovered tool: {name}"),
                "{}",
                "low_risk",
                3,
                0.5,
                r#"{"auto_registered": true, "source": "discovery"}"#,
            )?;
            registered.push(id);
        }

        Ok(registered)
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
        assert!(!id.is_empty());

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
    fn test_list_by_type() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        registry
            .register_tool(
                "cargo_check",
                "command",
                "Run cargo check",
                "{}",
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
                "OpenAI chat",
                "{}",
                "medium_risk",
                2,
                0.7,
                "{}",
            )
            .unwrap();
        let rows = registry.list_by_type("command", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "cargo_check");
    }

    #[test]
    fn test_list_by_type_no_match() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        registry
            .register_tool(
                "cargo_check",
                "command",
                "Run cargo check",
                "{}",
                "low_risk",
                3,
                0.9,
                "{}",
            )
            .unwrap();
        let rows = registry.list_by_type("nonexistent", 10).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_record_and_query_discovery() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        let id = registry
            .record_discovery("test-source", "cargo_check")
            .unwrap();
        assert!(!id.is_empty());
        let rows = registry.query_discovery("test-source", 10).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn test_discover_tools_no_skills_dir() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("tool_registry.db")).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        let result = registry.discover_tools("test", dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discover_tools_with_manifest() {
        let dir = tempdir().unwrap();
        let skills_dir = dir.path().join(".agents/skills/test-skill");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("manifest.json"),
            r#"{"tools": [{"name": "cargo_check"}, {"name": "lint"}]}"#,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("tool_registry.db")).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        let tools = registry.discover_tools("test", dir.path()).await.unwrap();
        assert!(tools.contains(&"cargo_check".to_string()));
        assert!(tools.contains(&"lint".to_string()));
    }

    #[tokio::test]
    async fn test_discover_tools_skips_duplicate() {
        let dir = tempdir().unwrap();
        let skills1 = dir.path().join(".agents/skills/skill-a");
        let skills2 = dir.path().join(".agents/skills/skill-b");
        std::fs::create_dir_all(&skills1).unwrap();
        std::fs::create_dir_all(&skills2).unwrap();
        std::fs::write(
            skills1.join("manifest.json"),
            r#"{"tools": [{"name": "cargo_check"}]}"#,
        )
        .unwrap();
        std::fs::write(
            skills2.join("manifest.json"),
            r#"{"tools": [{"name": "cargo_check"}, {"name": "lint"}]}"#,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("tool_registry.db")).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        let tools = registry.discover_tools("test", dir.path()).await.unwrap();
        assert_eq!(tools.iter().filter(|t| *t == "cargo_check").count(), 1);
        assert!(tools.contains(&"lint".to_string()));
    }

    #[tokio::test]
    async fn test_discover_tools_invalid_manifest() {
        let dir = tempdir().unwrap();
        let skills_dir = dir.path().join(".agents/skills/bad-skill");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(skills_dir.join("manifest.json"), "not valid json").unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("tool_registry.db")).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();
        let result = registry.discover_tools("test", dir.path()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tools_finds_known_binaries() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let results = registry
            .validate_tools(&["cargo_check".to_string(), "git_diff".to_string()])
            .unwrap();
        assert_eq!(results.len(), 2);

        // cargo_check maps to "cargo" which should exist
        assert!(
            results
                .iter()
                .any(|(name, ok, _)| name == "cargo_check" && *ok)
        );
        // git_diff maps to "git" which should exist
        assert!(
            results
                .iter()
                .any(|(name, ok, _)| name == "git_diff" && *ok)
        );
    }

    #[test]
    fn test_validate_tools_unknown_binary() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let results = registry
            .validate_tools(&["nonexistent_tool_xyz".to_string()])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].1);
        assert!(results[0].2.is_none());
    }

    #[test]
    fn test_auto_register_discovered() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let tools = vec!["new_tool".to_string(), "another_tool".to_string()];
        let registered = registry.auto_register_discovered(&tools).unwrap();
        assert_eq!(registered.len(), 2);

        // Verify they were registered
        let all_tools = registry.list_all(10).unwrap();
        assert!(all_tools.iter().any(|t| t.name == "new_tool"));
        assert!(all_tools.iter().any(|t| t.name == "another_tool"));
    }

    #[test]
    fn test_auto_register_skips_existing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        // Pre-register a tool
        registry
            .register_tool(
                "existing_tool",
                "command",
                "Pre-registered tool",
                "{}",
                "low_risk",
                3,
                0.9,
                "{}",
            )
            .unwrap();

        let tools = vec!["existing_tool".to_string(), "new_tool".to_string()];
        let registered = registry.auto_register_discovered(&tools).unwrap();
        assert_eq!(registered.len(), 1);

        // Verify only new_tool was registered (existing_tool was skipped)
        let all_tools = registry.list_all(10).unwrap();
        assert!(all_tools.iter().any(|t| t.name == "existing_tool"));
        assert!(all_tools.iter().any(|t| t.name == "new_tool"));
    }
}
