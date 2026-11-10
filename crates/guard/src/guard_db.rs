use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardDbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("database path not available: {0}")]
    PathError(String),

    #[error("schema initialization failed: {0}")]
    SchemaError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(
        "Unknown action status '{value}' for action {action_id} — \
         possible data corruption or schema mismatch"
    )]
    UnknownStatus { value: String, action_id: String },

    #[error("Schema version mismatch: database has v{found}, expected v{expected}")]
    SchemaVersionMismatch {
        found: String,
        expected: &'static str,
    },
}

/// Manages the guard database connection and schema initialization.
/// Uses a separate DB file from mirror-log to maintain detection/action separation.
#[derive(Clone)]
pub struct GuardDb {
    conn: Arc<Mutex<Connection>>,
}

impl GuardDb {
    /// Open or create the guard database at the given path.
    /// Default path is `guard.db` in the same directory as the mirror database.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GuardDbError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        // Enable WAL mode for concurrent access — allows multiple readers and one writer
        // simultaneously. Essential when crabjar processes run concurrently (fleet/MCP).
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        db.verify_schema_version()?;
        db.validate_rehydration()?;
        Ok(db)
    }

    /// Derive guard DB path from mirror DB path by replacing filename with `guard.db`.
    pub fn from_mirror_path(mirror_path: impl AsRef<Path>) -> PathBuf {
        let p = mirror_path.as_ref();
        let mut guard_path = p.parent().unwrap_or(Path::new(".")).to_path_buf();
        guard_path.push("guard.db");
        guard_path
    }

    /// Open guard DB co-located with mirror DB.
    pub fn co_located(mirror_path: impl AsRef<Path>) -> Result<Self, GuardDbError> {
        let guard_path = Self::from_mirror_path(mirror_path);
        Self::open(guard_path)
    }

    /// Expected schema version — must match the value seeded in schema.sql.
    const EXPECTED_SCHEMA_VERSION: &'static str = "3";

    fn init_schema(&self) -> Result<(), GuardDbError> {
        let schema = include_str!("schema.sql");
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(schema)?;
        Ok(())
    }

    /// Verify the database schema version matches expectations.
    /// Fails loudly if the schema is missing, unreadable, or at an unexpected version.
    fn verify_schema_version(&self) -> Result<(), GuardDbError> {
        let conn = self.conn.lock().unwrap();
        let found: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'version'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GuardDbError::SchemaVersionMismatch {
                    found: "missing".into(),
                    expected: Self::EXPECTED_SCHEMA_VERSION,
                },
                other => GuardDbError::Sqlite(other),
            })?;

        if found != Self::EXPECTED_SCHEMA_VERSION {
            return Err(GuardDbError::SchemaVersionMismatch {
                found,
                expected: Self::EXPECTED_SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    /// Phase 3: Rehydration validation on startup.
    ///
    /// Validates that all action_requests have consistent transition logs. This catches
    /// corruption or inconsistent state before it propagates into the execution pipeline.
    ///
    /// Uses the existing `read_action_request_validated` function which checks:
    /// - The first transition starts from 'none' (or a valid initial status)
    /// - Each subsequent transition's from_status matches the previous to_status
    /// - The final to_status matches the action_request's current status
    ///
    /// Logs warnings for inconsistencies but does not fail — the database should still
    /// be usable even if some records are corrupted. Corrupted records can be investigated
    /// via the transition log and manually repaired if needed.
    fn validate_rehydration(&self) -> Result<(), GuardDbError> {
        let conn = self.conn.lock().unwrap();
        let ids: Vec<String> = conn
            .prepare("SELECT id FROM action_requests")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;

        if ids.is_empty() {
            return Ok(());
        }

        let mut inconsistent = 0usize;
        for id in &ids {
            match Self::read_action_request_validated_on_conn(&conn, id) {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(action_id = %id, error = %e, "Rehydration validation: inconsistent action request");
                    inconsistent += 1;
                }
            }
        }

        if inconsistent > 0 {
            tracing::warn!(
                total = ids.len(),
                inconsistent,
                "Rehydration validation complete: {} of {} action requests have transition inconsistencies",
                inconsistent,
                ids.len()
            );
        } else {
            tracing::info!(
                total = ids.len(),
                "Rehydration validation complete: all action requests consistent"
            );
        }

        Ok(())
    }

    /// Get a guarded reference to the connection.
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_rehydration_validation_passes_on_consistent_db() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("guard_test.db");
        // Opening a fresh DB should pass rehydration validation (no action requests)
        let result = GuardDb::open(&db_path);
        assert!(result.is_ok(), "Fresh DB should open successfully");
    }

    #[test]
    fn test_rehydration_validation_flags_inconsistent_state() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("guard_inconsistent.db");
        let db = GuardDb::open(&db_path).unwrap();

        // Create an action request with a valid transition log
        db.persist_action_request_with_scope(
            "test-consistent",
            None,
            None,
            "run_command",
            "ls",
            2,
            0.5,
            None,
            None,
        )
        .unwrap();

        // Manually corrupt the state: set status to 'executed' without a transition log entry
        let conn = db.conn();
        conn.execute(
            "UPDATE action_requests SET status = 'executed' WHERE id = 'test-consistent'",
            [],
        )
        .unwrap();
        drop(conn);

        // Now reopen the DB — rehydration validation should flag the inconsistency
        let db2 = GuardDb::open(&db_path).unwrap();

        // The validated read should fail for this action
        let result = db2.read_action_request_validated("test-consistent");
        assert!(result.is_err(), "Should detect state mismatch");
    }
}
