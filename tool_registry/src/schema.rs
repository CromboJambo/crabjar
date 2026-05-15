use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolRegistrySchemaError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("schema initialization failed: {0}")]
    SchemaError(String),
}

/// Tool registry DDL schema. Structured agent tool registry with rig/mistral.rs patterns.
pub const TOOL_REGISTRY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL CHECK (type IN ('command', 'mcp', 'llm', 'vector_store', 'embedding', 'rag')),
    description TEXT NOT NULL DEFAULT '',
    schema TEXT NOT NULL DEFAULT '',
    execution_policy TEXT NOT NULL DEFAULT '',
    trust_layer INTEGER NOT NULL DEFAULT 0,
    confidence REAL NOT NULL DEFAULT 1.0 CHECK (confidence >= 0.0 AND confidence <= 1.0),
    registered_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_used INTEGER NOT NULL DEFAULT (unixepoch()),
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_tools_name ON tools(name);
CREATE INDEX IF NOT EXISTS idx_tools_type ON tools(type);
CREATE INDEX IF NOT EXISTS idx_tools_trust ON tools(trust_layer);
CREATE INDEX IF NOT EXISTS idx_tools_time ON tools(last_used DESC);

CREATE TABLE IF NOT EXISTS tool_usage (
    id TEXT PRIMARY KEY,
    tool_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    call_count INTEGER NOT NULL DEFAULT 0,
    success_rate REAL NOT NULL DEFAULT 1.0 CHECK (success_rate >= 0.0 AND success_rate <= 1.0),
    avg_latency REAL NOT NULL DEFAULT 0.0,
    last_call INTEGER NOT NULL DEFAULT (unixepoch()),
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_usage_tool ON tool_usage(tool_id);
CREATE INDEX IF NOT EXISTS idx_usage_session ON tool_usage(session_id);

CREATE TABLE IF NOT EXISTS tool_discovery (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    discovery_at INTEGER NOT NULL DEFAULT (unixepoch()),
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_discovery_source ON tool_discovery(source);
CREATE INDEX IF NOT EXISTS idx_discovery_time ON tool_discovery(discovery_at DESC);

CREATE TABLE IF NOT EXISTS schema_versions (
    version INTEGER PRIMARY KEY,
    applied TEXT NOT NULL DEFAULT (datetime('now')),
    note TEXT
);
"#;

pub fn init_db(conn: &Connection) -> Result<(), ToolRegistrySchemaError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(TOOL_REGISTRY_SCHEMA)?;
    Ok(())
}

pub fn register_tool(
    conn: &Connection,
    name: &str,
    type: &str,
    description: &str,
    schema: &str,
    execution_policy: &str,
    trust_layer: u32,
    confidence: f64,
    metadata: &str,
) -> Result<String, ToolRegistrySchemaError> {
    let id = uuid::Uuid::new_v4().to_string();
    let confidence = confidence.clamp(0.0, 1.0);

    conn.execute(
        "INSERT INTO tools (id, name, type, description, schema, execution_policy, trust_layer, confidence, registered_at, last_used, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            name,
            type,
            description,
            schema,
            execution_policy,
            trust_layer,
            confidence,
            chrono::Utc::now().timestamp(),
            chrono::Utc::now().timestamp(),
            metadata,
        ],
    )?;

    Ok(id)
}

pub fn query_tool(
    conn: &Connection,
    name: &str,
) -> Result<Option<ToolRow>, ToolRegistrySchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, description, schema, execution_policy, trust_layer, confidence, registered_at, last_used, metadata FROM tools
         WHERE name = ?1 LIMIT 1",
    )?;

    let row = stmt.query_row(params![name], |row| {
        let metadata_str: String = row.get(10)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();

        Ok(ToolRow {
            id: row.get(0)?,
            name: row.get(1)?,
            type: row.get(2)?,
            description: row.get(3)?,
            schema: row.get(4)?,
            execution_policy: row.get(5)?,
            trust_layer: row.get(6)?,
            confidence: row.get(7)?,
            registered_at: row.get(8)?,
            last_used: row.get(9)?,
            metadata,
        })
    });

    match row {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(ToolRegistrySchemaError::Sqlite(err)),
    }
}

pub fn list_all_tools(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<ToolRow>, ToolRegistrySchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, description, schema, execution_policy, trust_layer, confidence, registered_at, last_used, metadata FROM tools
         ORDER BY last_used DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        let metadata_str: String = row.get(10)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();

        Ok(ToolRow {
            id: row.get(0)?,
            name: row.get(1)?,
            type: row.get(2)?,
            description: row.get(3)?,
            schema: row.get(4)?,
            execution_policy: row.get(5)?,
            trust_layer: row.get(6)?,
            confidence: row.get(7)?,
            registered_at: row.get(8)?,
            last_used: row.get(9)?,
            metadata,
        })
    })?;

    let results = rows
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn list_tools_by_type(
    conn: &Connection,
    type: &str,
    limit: usize,
) -> Result<Vec<ToolRow>, ToolRegistrySchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, description, schema, execution_policy, trust_layer, confidence, registered_at, last_used, metadata FROM tools
         WHERE type = ?1
         ORDER BY last_used DESC LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![type, limit as i64], |row| {
        let metadata_str: String = row.get(10)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap_or_default();

        Ok(ToolRow {
            id: row.get(0)?,
            name: row.get(1)?,
            type: row.get(2)?,
            description: row.get(3)?,
            schema: row.get(4)?,
            execution_policy: row.get(5)?,
            trust_layer: row.get(6)?,
            confidence: row.get(7)?,
            registered_at: row.get(8)?,
            last_used: row.get(9)?,
            metadata,
        })
    })?;

    let results = rows
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn record_tool_usage(
    conn: &Connection,
    tool_id: &str,
    session_id: &str,
    call_count: i32,
    success_rate: f64,
    avg_latency: f64,
) -> Result<(), ToolRegistrySchemaError> {
    let id = uuid::Uuid::new_v4().to_string();
    let success_rate = success_rate.clamp(0.0, 1.0);
    let avg_latency = avg_latency.max(0.0);

    conn.execute(
        "INSERT INTO tool_usage (id, tool_id, session_id, call_count, success_rate, avg_latency, last_call, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            tool_id,
            session_id,
            call_count,
            success_rate,
            avg_latency,
            chrono::Utc::now().timestamp(),
            "{}",
        ],
    )?;

    Ok(())
}

pub fn query_tool_usage(
    conn: &Connection,
    tool_id: &str,
) -> Result<Vec<UsageRow>, ToolRegistrySchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, tool_id, session_id, call_count, success_rate, avg_latency, last_call FROM tool_usage
         WHERE tool_id = ?1",
    )?;

    let rows = stmt.query_map(params![tool_id], |row| {
        Ok(UsageRow {
            id: row.get(0)?,
            tool_id: row.get(1)?,
            session_id: row.get(2)?,
            call_count: row.get(3)?,
            success_rate: row.get(4)?,
            avg_latency: row.get(5)?,
            last_call: row.get(6)?,
        })
    })?;

    let results = rows
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn record_tool_discovery(
    conn: &Connection,
    source: &str,
    tool_name: &str,
) -> Result<String, ToolRegistrySchemaError> {
    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO tool_discovery (id, source, tool_name, metadata)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            id,
            source,
            tool_name,
            "{}",
        ],
    )?;

    Ok(id)
}

pub fn query_discovery(
    conn: &Connection,
    source: &str,
    limit: usize,
) -> Result<Vec<DiscoveryRow>, ToolRegistrySchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, source, tool_name, discovery_at FROM tool_discovery
         WHERE source = ?1
         ORDER BY discovery_at DESC LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![source, limit as i64], |row| {
        Ok(DiscoveryRow {
            id: row.get(0)?,
            source: row.get(1)?,
            tool_name: row.get(2)?,
            discovery_at: row.get(3)?,
        })
    })?;

    let results = rows
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// A single tool row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRow {
    pub id: String,
    pub name: String,
    pub type: String,
    pub description: String,
    pub schema: String,
    pub execution_policy: String,
    pub trust_layer: u32,
    pub confidence: f64,
    pub registered_at: i64,
    pub last_used: i64,
    pub metadata: serde_json::Value,
}

/// A single tool usage row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRow {
    pub id: String,
    pub tool_id: String,
    pub session_id: String,
    pub call_count: i32,
    pub success_rate: f64,
    pub avg_latency: f64,
    pub last_call: i64,
}

/// A single tool discovery row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRow {
    pub id: String,
    pub source: String,
    pub tool_name: String,
    pub discovery_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_db_creates_tables() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();
        assert!(db_path.exists());

        let count: i64 = conn
            .query_row("SELECT count(*) FROM tools", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_register_tool() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = register_tool(
            &conn,
            "cargo_check",
            "command",
            "Run cargo check on workspace",
            '{"tool": "cargo", "args": ["check", "--workspace"]}',
            "low_risk",
            3,
            0.9,
            "{}",
        ).unwrap();

        assert!(!id.is_empty());

        let row = query_tool(&conn, "cargo_check").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().name, "cargo_check");
        assert_eq!(row.unwrap().trust_layer, 3);
    }

    #[test]
    fn test_list_all_tools() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        register_tool(
            &conn,
            "cargo_check",
            "command",
            "Run cargo check on workspace",
            '{"tool": "cargo", "args": ["check", "--workspace"]}',
            "low_risk",
            3,
            0.9,
            "{}",
        ).unwrap();

        register_tool(
            &conn,
            "openai_chat",
            "llm",
            "OpenAI chat completion",
            '{"provider": "openai", "model": "gpt-4o"}',
            "medium_risk",
            2,
            0.7,
            "{}",
        ).unwrap();

        let rows = list_all_tools(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_list_tools_by_type() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        register_tool(
            &conn,
            "cargo_check",
            "command",
            "Run cargo check on workspace",
            '{"tool": "cargo", "args": ["check", "--workspace"]}',
            "low_risk",
            3,
            0.9,
            "{}",
        ).unwrap();

        register_tool(
            &conn,
            "openai_chat",
            "llm",
            "OpenAI chat completion",
            '{"provider": "openai", "model": "gpt-4o"}',
            "medium_risk",
            2,
            0.7,
            "{}",
        ).unwrap();

        let rows = list_tools_by_type(&conn, "command", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "cargo_check");
    }

    #[test]
    fn test_record_tool_usage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let tool_id = register_tool(
            &conn,
            "cargo_check",
            "command",
            "Run cargo check on workspace",
            '{"tool": "cargo", "args": ["check", "--workspace"]}',
            "low_risk",
            3,
            0.9,
            "{}",
        ).unwrap();

        record_tool_usage(
            &conn,
            &tool_id,
            "session-1",
            5,
            1.0,
            0.5,
        ).unwrap();

        let rows = query_tool_usage(&conn, &tool_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].call_count, 5);
        assert_eq!(rows[0].success_rate, 1.0);
    }

    #[test]
    fn test_record_tool_discovery() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = record_tool_discovery(
            &conn,
            "aur_search",
            "spotify",
        ).unwrap();

        let rows = query_discovery(&conn, "aur_search", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tool_name, "spotify");
    }
}
