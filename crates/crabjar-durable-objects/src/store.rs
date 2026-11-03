//! SQLite-backed checkpoint metadata store.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::config::CheckpointConfig;
use crate::error::{CheckpointError, CheckpointResult};

/// Metadata for a single checkpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckpointMeta {
    pub id: String,
    pub task_id: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub size_bytes: i64,
    pub status: CheckpointStatus,
}

/// Status of a checkpoint.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum CheckpointStatus {
    Active,
    Pruned,
    Failed,
}

impl CheckpointStatus {
    fn as_str(&self) -> &'static str {
        match self {
            CheckpointStatus::Active => "active",
            CheckpointStatus::Pruned => "pruned",
            CheckpointStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "pruned" => CheckpointStatus::Pruned,
            "failed" => CheckpointStatus::Failed,
            _ => CheckpointStatus::Active,
        }
    }
}

/// SQLite-backed checkpoint metadata store.
pub struct CheckpointStore {
    db: Connection,
    config: CheckpointConfig,
}

impl CheckpointStore {
    /// Open or create a checkpoint store at the given path.
    pub fn open(path: impl AsRef<std::path::Path>) -> CheckpointResult<Self> {
        let db = Connection::open(&path)?;

        // Initialize schema
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS checkpoints (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            size_bytes INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            payload_path TEXT,
            error_message TEXT
        );",
        )?;

        db.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_task ON checkpoints(task_id);",
        )?;
        db.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_status ON checkpoints(status);",
        )?;

        Ok(Self {
            db,
            config: CheckpointConfig::new(path.as_ref().to_string_lossy()),
        })
    }

    /// Create a checkpoint store with custom configuration
    pub fn new(config: &CheckpointConfig) -> CheckpointResult<Self> {
        let db = Connection::open(&config.db_path)?;

        // Initialize schema
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS checkpoints (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            size_bytes INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            payload_path TEXT,
            error_message TEXT
        );",
        )?;

        db.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_task ON checkpoints(task_id);",
        )?;
        db.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_status ON checkpoints(status);",
        )?;

        Ok(Self {
            db,
            config: config.clone(),
        })
    }

    /// Create a new checkpoint record.
    pub fn create_checkpoint(
        &self,
        task_id: &str,
        payload: serde_json::Value,
        payload_path: Option<&str>,
    ) -> CheckpointResult<String> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        // Calculate size from payload
        let payload_str = payload.to_string();
        let size_bytes = payload_str.len() as i64;

        self.db.execute(
            "INSERT INTO checkpoints (id, task_id, created_at, size_bytes, status, payload_path) VALUES (?, ?, ?, ?, 'active', ?)",
            params![id, task_id, created_at.to_rfc3339(), size_bytes, payload_path],
        )?;

        Ok(id)
    }

    /// Get checkpoint metadata by ID.
    pub fn get_checkpoint(&self, id: &str) -> CheckpointResult<Option<CheckpointMeta>> {
        let mut stmt = self.db.prepare(
            "SELECT id, task_id, created_at, size_bytes, status FROM checkpoints WHERE id = ?",
        )?;
        let mut rows = stmt.query(params![id])?;

        loop {
            match rows.next() {
                Ok(Some(row)) => {
                    let created_at_str: String = row.get(2)?;
                    let status = CheckpointStatus::from_str(&row.get::<_, String>(4)?);
                    return Ok(Some(CheckpointMeta {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        created_at: DateTime::parse_from_rfc3339(&created_at_str)
                            .map_err(|e| CheckpointError::Validation(e.to_string()))?
                            .with_timezone(&Utc),
                        size_bytes: row.get(3)?,
                        status,
                    }));
                }
                Ok(None) => return Ok(None),
                Err(e) => return Err(CheckpointError::DbError(e)),
            }
        }
    }

    /// Get the latest checkpoint for a task.
    pub fn get_latest_checkpoint(&self, task_id: &str) -> CheckpointResult<Option<CheckpointMeta>> {
        let mut stmt = self.db.prepare("SELECT id, task_id, created_at, size_bytes, status FROM checkpoints WHERE task_id = ? AND status = 'active' ORDER BY created_at DESC LIMIT 1")?;
        let mut rows = stmt.query(params![task_id])?;

        loop {
            match rows.next() {
                Ok(Some(row)) => {
                    let created_at_str: String = row.get(2)?;
                    let status = CheckpointStatus::from_str(&row.get::<_, String>(4)?);
                    return Ok(Some(CheckpointMeta {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        created_at: DateTime::parse_from_rfc3339(&created_at_str)
                            .map_err(|e| CheckpointError::Validation(e.to_string()))?
                            .with_timezone(&Utc),
                        size_bytes: row.get(3)?,
                        status,
                    }));
                }
                Ok(None) => return Ok(None),
                Err(e) => return Err(CheckpointError::DbError(e)),
            }
        }
    }

    /// List all checkpoints for a task.
    pub fn list_checkpoints(&self, task_id: &str) -> CheckpointResult<Vec<CheckpointMeta>> {
        let mut stmt = self.db.prepare("SELECT id, task_id, created_at, size_bytes, status FROM checkpoints WHERE task_id = ? AND status = 'active' ORDER BY created_at DESC")?;
        let mut rows = stmt.query(params![task_id])?;

        let mut checkpoints = Vec::new();
        loop {
            match rows.next() {
                Ok(Some(row)) => {
                    let created_at_str: String = row.get(2)?;
                    let status = CheckpointStatus::from_str(&row.get::<_, String>(4)?);
                    checkpoints.push(CheckpointMeta {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        created_at: DateTime::parse_from_rfc3339(&created_at_str)
                            .map_err(|e| CheckpointError::Validation(e.to_string()))?
                            .with_timezone(&Utc),
                        size_bytes: row.get(3)?,
                        status,
                    });
                }
                Ok(None) => break,
                Err(e) => return Err(CheckpointError::DbError(e)),
            }
        }

        Ok(checkpoints)
    }

    /// Prune old checkpoints beyond retention policy.
    pub fn prune_old_checkpoints(&self, task_id: &str) -> CheckpointResult<u32> {
        let checkpoints = self.list_checkpoints(task_id)?;
        let mut pruned = 0;

        for cp in checkpoints
            .iter()
            .skip(self.config.retention_count as usize)
        {
            self.db.execute(
                "UPDATE checkpoints SET status = 'pruned' WHERE id = ?",
                params![cp.id],
            )?;
            pruned += 1;
        }

        Ok(pruned)
    }
}
