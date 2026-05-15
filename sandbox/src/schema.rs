use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxSchemaError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("schema initialization failed: {0}")]
    SchemaError(String),
}

/// Sandbox configuration DDL schema.
pub const SANDBOX_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_sandbox (
    id TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL UNIQUE,
    isolation_type TEXT NOT NULL CHECK (isolation_type IN ('unix_user', 'systemd_nspawn', 'cgroup')),
    home_path TEXT NOT NULL DEFAULT '',
    shell_config TEXT NOT NULL DEFAULT '',
    cache_dirs TEXT NOT NULL DEFAULT '',
    network_egress TEXT NOT NULL DEFAULT '',
    resource_limits TEXT NOT NULL DEFAULT '',
    sudo_policy TEXT NOT NULL DEFAULT '',
    mount_scopes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_used INTEGER NOT NULL DEFAULT (unixepoch()),
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_sandbox_agent ON agent_sandbox(agent_name);
CREATE INDEX IF NOT EXISTS idx_sandbox_type ON agent_sandbox(isolation_type);
CREATE INDEX IF NOT EXISTS idx_sandbox_time ON agent_sandbox(last_used DESC);

CREATE TABLE IF NOT EXISTS sandbox_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO sandbox_config (key, value) VALUES
    ('default_isolation_type', 'unix_user'),
    ('network_egress_policy', 'restricted'),
    ('sudo_policy', 'no_sudo'),
    ('default_resource_limits', 'cpu=2,memory=4g');

CREATE TABLE IF NOT EXISTS schema_versions (
    version INTEGER PRIMARY KEY,
    applied TEXT NOT NULL DEFAULT (datetime('now')),
    note TEXT
);
"#;

pub fn init_db(conn: &Connection) -> Result<(), SandboxSchemaError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SANDBOX_SCHEMA)?;
    Ok(())
}

pub fn create_agent_sandbox(
    conn: &Connection,
    agent_name: &str,
    isolation_type: &str,
    home_path: &str,
    shell_config: &str,
    cache_dirs: &str,
    network_egress: &str,
    resource_limits: &str,
    sudo_policy: &str,
    mount_scopes: &str,
) -> Result<String, SandboxSchemaError> {
    let id = uuid::Uuid::new_v4().to_string();
    let cache_dirs_json = serde_json::to_string(&cache_dirs.split(',').collect::<Vec<_>>()).map_err(|e| SandboxSchemaError::Json(e))?;
    let mount_scopes_json = serde_json::to_string(&mount_scopes.split(',').collect::<Vec<_>>()).map_err(|e| SandboxSchemaError::Json(e))?;

    conn.execute(
        "INSERT INTO agent_sandbox (id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            agent_name,
            isolation_type,
            home_path,
            shell_config,
            cache_dirs_json,
            network_egress,
            resource_limits,
            sudo_policy,
            mount_scopes_json,
            "{}",
        ],
    )?;

    Ok(id)
}

pub fn query_agent_sandbox(
    conn: &Connection,
    agent_name: &str,
) -> Result<Option<SandboxRow>, SandboxSchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, last_used FROM agent_sandbox
         WHERE agent_name = ?1 LIMIT 1",
    )?;

    let row = stmt.query_row(params![agent_name], |row| {
        let cache_dirs_str: String = row.get(6)?;
        let cache_dirs: Vec<String> = serde_json::from_str(&cache_dirs_str).unwrap_or_default();
        let mount_scopes_str: String = row.get(9)?;
        let mount_scopes: Vec<String> = serde_json::from_str(&mount_scopes_str).unwrap_or_default();

        Ok(SandboxRow {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            isolation_type: row.get(2)?,
            home_path: row.get(3)?,
            shell_config: row.get(4)?,
            cache_dirs,
            network_egress: row.get(5)?,
            resource_limits: row.get(6)?,
            sudo_policy: row.get(7)?,
            mount_scopes,
            created_at: row.get(8)?,
            last_used: row.get(9)?,
        })
    });

    match row {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(SandboxSchemaError::Sqlite(err)),
    }
}

pub fn list_all_sandboxes(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<SandboxRow>, SandboxSchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, last_used FROM agent_sandbox
         ORDER BY last_used DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        let cache_dirs_str: String = row.get(6)?;
        let cache_dirs: Vec<String> = serde_json::from_str(&cache_dirs_str).unwrap_or_default();
        let mount_scopes_str: String = row.get(9)?;
        let mount_scopes: Vec<String> = serde_json::from_str(&mount_scopes_str).unwrap_or_default();

        Ok(SandboxRow {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            isolation_type: row.get(2)?,
            home_path: row.get(3)?,
            shell_config: row.get(4)?,
            cache_dirs,
            network_egress: row.get(5)?,
            resource_limits: row.get(6)?,
            sudo_policy: row.get(7)?,
            mount_scopes,
            created_at: row.get(8)?,
            last_used: row.get(9)?,
        })
    })?;

    let results = rows
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// A single agent sandbox row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRow {
    pub id: String,
    pub agent_name: String,
    pub isolation_type: String,
    pub home_path: String,
    pub shell_config: String,
    pub cache_dirs: Vec<String>,
    pub network_egress: String,
    pub resource_limits: String,
    pub sudo_policy: String,
    pub mount_scopes: Vec<String>,
    pub created_at: i64,
    pub last_used: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_db_creates_tables() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();
        assert!(db_path.exists());

        let count: i64 = conn
            .query_row("SELECT count(*) FROM agent_sandbox", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_agent_sandbox() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = create_agent_sandbox(
            &conn,
            "agent-coder",
            "unix_user",
            "/home/agent-coder",
            "nushell",
            "/home/agent-coder/.cache",
            "restricted",
            "cpu=2,memory=4g",
            "no_sudo",
            "/repo:/tmp",
        ).unwrap();

        assert!(!id.is_empty());

        let row = query_agent_sandbox(&conn, "agent-coder").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().agent_name, "agent-coder");
        assert_eq!(row.unwrap().isolation_type, "unix_user");
    }

    #[test]
    fn test_list_all_sandboxes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        create_agent_sandbox(
            &conn,
            "agent-coder",
            "unix_user",
            "/home/agent-coder",
            "nushell",
            "/home/agent-coder/.cache",
            "restricted",
            "cpu=2,memory=4g",
            "no_sudo",
            "/repo:/tmp",
        ).unwrap();

        create_agent_sandbox(
            &conn,
            "agent-research",
            "systemd_nspawn",
            "",
            "",
            "",
            "open",
            "cpu=4,memory=8g",
            "limited_sudo",
            "/repo:/tmp:/data",
        ).unwrap();

        let rows = list_all_sandboxes(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
    }
}
