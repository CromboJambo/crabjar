use crate::error::SandboxSchemaError;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

/// Agent sandbox DDL schema.
pub const SANDBOX_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_sandboxes (
    id TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL,
    isolation_type TEXT NOT NULL DEFAULT 'unix_user',
    home_path TEXT NOT NULL DEFAULT '',
    shell_config TEXT NOT NULL DEFAULT '',
    cache_dirs TEXT NOT NULL DEFAULT '',
    network_egress TEXT NOT NULL DEFAULT 'restricted',
    resource_limits TEXT NOT NULL DEFAULT '',
    sudo_policy TEXT NOT NULL DEFAULT 'no_sudo',
    mount_scopes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    active INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_sandboxes_agent ON agent_sandboxes(agent_name);
CREATE INDEX IF NOT EXISTS idx_sandboxes_type ON agent_sandboxes(isolation_type);
CREATE INDEX IF NOT EXISTS idx_sandboxes_time ON agent_sandboxes(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sandboxes_active ON agent_sandboxes(active);

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

    conn.execute(
        "INSERT INTO agent_sandboxes (id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            agent_name,
            isolation_type,
            home_path,
            shell_config,
            cache_dirs,
            network_egress,
            resource_limits,
            sudo_policy,
            mount_scopes,
            chrono::Utc::now().timestamp(),
            1,
        ],
    )?;

    Ok(id)
}

pub fn query_agent_sandbox(
    conn: &Connection,
    agent_name: &str,
) -> Result<Option<SandboxRow>, SandboxSchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, active FROM agent_sandboxes
         WHERE agent_name = ?1 AND active = 1 LIMIT 1",
    )?;

    match stmt.query_row(params![agent_name], |row| {
        Ok(SandboxRow {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            isolation_type: row.get(2)?,
            home_path: row.get(3)?,
            shell_config: row.get(4)?,
            cache_dirs: row.get(5)?,
            network_egress: row.get(6)?,
            resource_limits: row.get(7)?,
            sudo_policy: row.get(8)?,
            mount_scopes: row.get(9)?,
            created_at: row.get(10)?,
            active: row.get(11)?,
        })
    }) {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(SandboxSchemaError::Sqlite(err)),
    }
}

pub fn list_all_sandboxes(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<SandboxRow>, SandboxSchemaError> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, active FROM agent_sandboxes
         WHERE active = 1
         ORDER BY created_at DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(SandboxRow {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            isolation_type: row.get(2)?,
            home_path: row.get(3)?,
            shell_config: row.get(4)?,
            cache_dirs: row.get(5)?,
            network_egress: row.get(6)?,
            resource_limits: row.get(7)?,
            sudo_policy: row.get(8)?,
            mount_scopes: row.get(9)?,
            created_at: row.get(10)?,
            active: row.get(11)?,
        })
    })?;

    let results = rows.collect::<Result<Vec<_>, _>>()?;

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
    pub cache_dirs: String,
    pub network_egress: String,
    pub resource_limits: String,
    pub sudo_policy: String,
    pub mount_scopes: String,
    pub created_at: i64,
    pub active: i32,
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
            .query_row("SELECT count(*) FROM agent_sandboxes", [], |r| r.get(0))
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
            "test-agent",
            "unix_user",
            "/home/test-agent",
            "bash",
            "/home/test-agent/.cache",
            "restricted",
            "cpu=1,memory=2g",
            "no_sudo",
            "/repo:/tmp",
        )
        .unwrap();

        assert!(!id.is_empty());

        let row = query_agent_sandbox(&conn, "test-agent").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().agent_name, "test-agent");
    }

    #[test]
    fn test_list_all_sandboxes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        create_agent_sandbox(
            &conn,
            "agent-a",
            "unix_user",
            "/home/agent-a",
            "bash",
            "",
            "restricted",
            "",
            "no_sudo",
            "",
        )
        .unwrap();

        create_agent_sandbox(
            &conn,
            "agent-b",
            "container",
            "/tmp/agent-b",
            "sh",
            "",
            "open",
            "",
            "limited",
            "",
        )
        .unwrap();

        let rows = list_all_sandboxes(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
    }
}
