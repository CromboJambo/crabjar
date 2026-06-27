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
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
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

    fn init_schema(&self) -> Result<(), GuardDbError> {
        let schema = include_str!("schema.sql");
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(schema)?;
        Ok(())
    }

    /// Get a guarded reference to the connection.
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

}
