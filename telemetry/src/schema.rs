use crate::error::FlightRecorderError;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

/// Flight recorder DDL schema. Append-only — never modify existing steps.
pub const FLIGHT_RECORDER_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS flight_records (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    command TEXT NOT NULL,
    cwd TEXT NOT NULL,
    args TEXT NOT NULL,
    exit_code INTEGER NOT NULL DEFAULT -1,
    stdout_hash TEXT NOT NULL DEFAULT '',
    stderr_hash TEXT NOT NULL DEFAULT '',
    stdout_len INTEGER NOT NULL DEFAULT 0,
    stderr_len INTEGER NOT NULL DEFAULT 0,
    git_dirty INTEGER NOT NULL DEFAULT 0,
    git_diff_count INTEGER NOT NULL DEFAULT 0,
    git_diff_hash TEXT NOT NULL DEFAULT '',
    reason TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT 'agent',
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_flight_records_session ON flight_records(session_id);
CREATE INDEX IF NOT EXISTS idx_flight_records_timestamp ON flight_records(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_flight_records_command ON flight_records(command);
CREATE INDEX IF NOT EXISTS idx_flight_records_exit ON flight_records(exit_code);
CREATE INDEX IF NOT EXISTS idx_flight_records_source ON flight_records(source);

CREATE TABLE IF NOT EXISTS session_checkpoint (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    repo_state_hash TEXT NOT NULL,
    dirty_tree_count INTEGER NOT NULL DEFAULT 0,
    checkpointed_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_session_checkpoint_session ON session_checkpoint(session_id);
CREATE INDEX IF NOT EXISTS idx_session_checkpoint_time ON session_checkpoint(checkpointed_at DESC);

CREATE TABLE IF NOT EXISTS command_transcript (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    stdout_line TEXT NOT NULL DEFAULT '',
    stderr_line TEXT NOT NULL DEFAULT '',
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    source TEXT NOT NULL DEFAULT 'agent'
);

CREATE INDEX IF NOT EXISTS idx_transcript_session ON command_transcript(session_id);
CREATE INDEX IF NOT EXISTS idx_transcript_command ON command_transcript(command_id);
CREATE INDEX IF NOT EXISTS idx_transcript_time ON command_transcript(timestamp DESC);

CREATE TABLE IF NOT EXISTS schema_versions (
    version INTEGER PRIMARY KEY,
    applied TEXT NOT NULL DEFAULT (datetime('now')),
    note TEXT
);
"#;

pub fn init_db(conn: &Connection) -> Result<(), FlightRecorderError> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(FLIGHT_RECORDER_SCHEMA)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn record_command(
    conn: &Connection,
    session_id: &str,
    command: &str,
    cwd: &str,
    args: &[String],
    exit_code: i32,
    stdout_hash: &str,
    stderr_hash: &str,
    stdout_len: usize,
    stderr_len: usize,
    git_dirty: i32,
    git_diff_count: i32,
    git_diff_hash: &str,
    reason: &str,
    source: &str,
) -> Result<String, FlightRecorderError> {
    let id = uuid::Uuid::new_v4().to_string();
    let args_json = serde_json::to_string(args).map_err(FlightRecorderError::Json)?;

    conn.execute(
        "INSERT INTO flight_records (id, session_id, command, cwd, args, exit_code, stdout_hash, stderr_hash, stdout_len, stderr_len, git_dirty, git_diff_count, git_diff_hash, reason, source, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            id,
            session_id,
            command,
            cwd,
            args_json,
            exit_code,
            stdout_hash,
            stderr_hash,
            stdout_len,
            stderr_len,
            git_dirty,
            git_diff_count,
            git_diff_hash,
            reason,
            source,
            "{}",
        ],
    )?;

    Ok(id)
}

pub fn record_transcript_line(
    conn: &Connection,
    session_id: &str,
    command_id: &str,
    line_number: i32,
    stdout_line: &str,
    stderr_line: &str,
) -> Result<(), FlightRecorderError> {
    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO command_transcript (id, session_id, command_id, line_number, stdout_line, stderr_line, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            session_id,
            command_id,
            line_number,
            stdout_line,
            stderr_line,
            "agent",
        ],
    )?;

    Ok(())
}

pub fn checkpoint_session(
    conn: &Connection,
    session_id: &str,
    repo_state_hash: &str,
    dirty_tree_count: i32,
) -> Result<String, FlightRecorderError> {
    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO session_checkpoint (id, session_id, repo_state_hash, dirty_tree_count, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            session_id,
            repo_state_hash,
            dirty_tree_count,
            "{}",
        ],
    )?;

    Ok(id)
}

pub fn query_flight_records(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<FlightRecordRow>, FlightRecorderError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, timestamp, command, cwd, args, exit_code, stdout_hash, stderr_hash, stdout_len, stderr_len, git_dirty, git_diff_count, git_diff_hash, reason, source FROM flight_records
         WHERE session_id = ?1
         ORDER BY timestamp DESC LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![session_id, limit as i64], |row| {
        let args_str: String = row.get(5)?;
        let args: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();

        Ok(FlightRecordRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            timestamp: row.get(2)?,
            command: row.get(3)?,
            cwd: row.get(4)?,
            args,
            exit_code: row.get(6)?,
            stdout_hash: row.get(7)?,
            stderr_hash: row.get(8)?,
            stdout_len: row.get(9)?,
            stderr_len: row.get(10)?,
            git_dirty: row.get(11)?,
            git_diff_count: row.get(12)?,
            git_diff_hash: row.get(13)?,
            reason: row.get(14)?,
            source: row.get(15)?,
        })
    })?;

    let results = rows.collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn query_session_checkpoints(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<CheckpointRow>, FlightRecorderError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, repo_state_hash, dirty_tree_count, checkpointed_at FROM session_checkpoint
         WHERE session_id = ?1
         ORDER BY checkpointed_at DESC LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![session_id, limit as i64], |row| {
        Ok(CheckpointRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            repo_state_hash: row.get(2)?,
            dirty_tree_count: row.get(3)?,
            checkpointed_at: row.get(4)?,
        })
    })?;

    let results = rows.collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn query_transcript(
    conn: &Connection,
    session_id: &str,
    command_id: &str,
    limit: usize,
) -> Result<Vec<TranscriptRow>, FlightRecorderError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, command_id, line_number, stdout_line, stderr_line, timestamp FROM command_transcript
         WHERE session_id = ?1 AND command_id = ?2
         ORDER BY line_number ASC LIMIT ?3",
    )?;

    let rows = stmt.query_map(params![session_id, command_id, limit as i64], |row| {
        Ok(TranscriptRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            command_id: row.get(2)?,
            line_number: row.get(3)?,
            stdout_line: row.get(4)?,
            stderr_line: row.get(5)?,
            timestamp: row.get(6)?,
        })
    })?;

    let results = rows.collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn list_all_records(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<FlightRecordRow>, FlightRecorderError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, timestamp, command, cwd, args, exit_code, stdout_hash, stderr_hash, stdout_len, stderr_len, git_dirty, git_diff_count, git_diff_hash, reason, source FROM flight_records
         ORDER BY timestamp DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        let args_str: String = row.get(5)?;
        let args: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();

        Ok(FlightRecordRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            timestamp: row.get(2)?,
            command: row.get(3)?,
            cwd: row.get(4)?,
            args,
            exit_code: row.get(6)?,
            stdout_hash: row.get(7)?,
            stderr_hash: row.get(8)?,
            stdout_len: row.get(9)?,
            stderr_len: row.get(10)?,
            git_dirty: row.get(11)?,
            git_diff_count: row.get(12)?,
            git_diff_hash: row.get(13)?,
            reason: row.get(14)?,
            source: row.get(15)?,
        })
    })?;

    let results = rows.collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// A single flight record row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightRecordRow {
    pub id: String,
    pub session_id: String,
    pub timestamp: String,
    pub command: String,
    pub cwd: String,
    pub args: Vec<String>,
    pub exit_code: i32,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub stdout_len: usize,
    pub stderr_len: usize,
    pub git_dirty: i32,
    pub git_diff_count: i32,
    pub git_diff_hash: String,
    pub reason: String,
    pub source: String,
}

/// A single session checkpoint row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRow {
    pub id: String,
    pub session_id: String,
    pub repo_state_hash: String,
    pub dirty_tree_count: i32,
    pub checkpointed_at: String,
}

/// A single transcript line row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptRow {
    pub id: String,
    pub session_id: String,
    pub command_id: String,
    pub line_number: i32,
    pub stdout_line: String,
    pub stderr_line: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_db_creates_tables() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();
        assert!(db_path.exists());

        let count: i64 = conn
            .query_row("SELECT count(*) FROM flight_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_record_command() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = record_command(
            &conn,
            "session-1",
            "cargo check",
            "/repo",
            &["--workspace".to_string()],
            0,
            "hash1",
            "hash2",
            100,
            0,
            0,
            0,
            "",
            "task: check workspace",
            "agent",
        )
        .unwrap();

        assert!(!id.is_empty());

        let rows = query_flight_records(&conn, "session-1", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].command, "cargo check");
        assert_eq!(rows[0].exit_code, 0);
    }

    #[test]
    fn test_checkpoint_session() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = checkpoint_session(&conn, "session-1", "abc123", 5).unwrap();

        let rows = query_session_checkpoints(&conn, "session-1", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].repo_state_hash, "abc123");
        assert_eq!(rows[0].dirty_tree_count, 5);
    }

    #[test]
    fn test_transcript_lines() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let cmd_id = record_command(
            &conn,
            "session-1",
            "cargo test",
            "/repo",
            &[].to_vec(),
            101,
            "hash1",
            "hash2",
            500,
            200,
            3,
            2,
            "diff_hash",
            "test: run tests",
            "agent",
        )
        .unwrap();

        record_transcript_line(&conn, "session-1", &cmd_id, 1, "test passed", "").unwrap();
        record_transcript_line(&conn, "session-1", &cmd_id, 2, "", "test failed in parser")
            .unwrap();

        let rows = query_transcript(&conn, "session-1", &cmd_id, 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stdout_line, "test passed");
        assert_eq!(rows[1].stderr_line, "test failed in parser");
    }

    #[test]
    fn test_list_all_records() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        record_command(
            &conn,
            "session-1",
            "cargo fmt",
            "/repo",
            &["--check".to_string()],
            0,
            "",
            "",
            0,
            0,
            0,
            0,
            "",
            "",
            "agent",
        )
        .unwrap();

        record_command(
            &conn,
            "session-2",
            "cargo clippy",
            "/repo",
            &["--workspace".to_string()],
            0,
            "",
            "",
            0,
            0,
            0,
            0,
            "",
            "",
            "agent",
        )
        .unwrap();

        let rows = list_all_records(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_command_with_stderr_failure() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        init_db(&conn).unwrap();

        let id = record_command(
            &conn,
            "session-1",
            "rm -rf /tmp/test",
            "/repo",
            &["-rf".to_string(), "/tmp/test".to_string()],
            1,
            "",
            "stderr_hash",
            0,
            50,
            0,
            0,
            "",
            "action: deleted test dir",
            "agent",
        )
        .unwrap();

        let rows = query_flight_records(&conn, "session-1", 10).unwrap();
        assert_eq!(rows[0].exit_code, 1);
        assert!(!rows[0].stderr_hash.is_empty());
        assert_eq!(rows[0].stderr_len, 50);
    }
}
