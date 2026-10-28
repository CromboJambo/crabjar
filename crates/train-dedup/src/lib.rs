//! Training data deduplication engine with occurrence tracking.
//!
//! Uses SHA-256 content hashing after normalization to detect duplicates,
//! with a SQLite registry that tracks occurrence frequency and timestamps.

use chrono::Utc;
use hex::encode;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error types for dedup operations.
#[derive(Debug, Error)]
pub enum DedupError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hash computation failed: {0}")]
    Hash(String),
}

/// Status of a training entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntryStatus {
    Pending,
    Clean,
    Duplicate,
    Error,
}

impl std::fmt::Display for EntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryStatus::Pending => write!(f, "pending"),
            EntryStatus::Clean => write!(f, "clean"),
            EntryStatus::Duplicate => write!(f, "duplicate"),
            EntryStatus::Error => write!(f, "error"),
        }
    }
}

impl From<&str> for EntryStatus {
    fn from(s: &str) -> Self {
        match s {
            "clean" => EntryStatus::Clean,
            "duplicate" => EntryStatus::Duplicate,
            "error" => EntryStatus::Error,
            _ => EntryStatus::Pending,
        }
    }
}

/// A training entry record from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEntry {
    pub id: i64,
    pub content_hash: String,
    pub original_path: String,
    pub processed_path: Option<String>,
    pub first_seen: chrono::DateTime<Utc>,
    pub last_seen: chrono::DateTime<Utc>,
    pub occurrences: i32,
    pub secrets_found: Option<Vec<String>>,
    pub status: EntryStatus,
}

/// Result of processing a file through the dedup engine.
#[derive(Debug)]
pub enum ProcessResult {
    /// New unique entry inserted into registry.
    New(TrainingEntry),
    /// Duplicate detected; occurrence count incremented.
    Duplicate(TrainingEntry),
}

/// The deduplication engine with SQLite backend.
pub struct DedupEngine {
    conn: Connection,
}

impl DedupEngine {
    /// Create a new dedup engine backed by the given SQLite database path.
    /// Initializes the schema if it doesn't exist.
    pub fn new(db_path: impl AsRef<std::path::Path>) -> Result<Self, DedupError> {
        let conn = Connection::open(db_path)?;
        let engine = Self { conn };
        engine.init_schema()?;
        Ok(engine)
    }

    fn init_schema(&self) -> Result<(), DedupError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS training_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_hash TEXT UNIQUE NOT NULL,
                original_path TEXT NOT NULL,
                processed_path TEXT,
                first_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                occurrences INTEGER NOT NULL DEFAULT 1,
                secrets_found TEXT,
                status TEXT NOT NULL DEFAULT 'pending'
            );

            CREATE INDEX IF NOT EXISTS idx_hash ON training_entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_status ON training_entries(status);
            CREATE INDEX IF NOT EXISTS idx_last_seen ON training_entries(last_seen);",
        )?;
        Ok(())
    }

    /// Normalize text for hashing: collapse whitespace, case-fold.
    fn normalize(text: &str) -> String {
        let folded = text.to_lowercase();
        let mut result = String::new();
        let mut prev_was_whitespace = false;

        for ch in folded.chars() {
            if ch.is_whitespace() {
                if !prev_was_whitespace {
                    result.push(' ');
                }
                prev_was_whitespace = true;
            } else {
                result.push(ch);
                prev_was_whitespace = false;
            }
        }

        result.trim().to_string()
    }

    /// Compute SHA-256 hash of normalized content.
    pub fn compute_hash(content: &str) -> String {
        let normalized = Self::normalize(content);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        encode(hasher.finalize())
    }

    /// Process a file through the dedup engine.
    ///
    /// Returns `ProcessResult::New` for unique content or `ProcessResult::Duplicate`
    /// if the content hash already exists in the registry.
    pub fn process_file(
        &self,
        original_path: impl AsRef<str>,
        content: &str,
        secrets_found: Option<Vec<String>>,
    ) -> Result<ProcessResult, DedupError> {
        let original_path = original_path.as_ref().to_string();
        let hash = Self::compute_hash(content);

        // Check for existing entry
        let existing = self.conn.query_row(
            "SELECT id, content_hash, original_path, processed_path, first_seen, last_seen, occurrences, secrets_found, status
             FROM training_entries WHERE content_hash = ?1",
            params![hash],
            |row| {
                Ok(TrainingEntry {
                    id: row.get(0)?,
                    content_hash: row.get(1)?,
                    original_path: row.get(2)?,
                    processed_path: row.get(3)?,
                    first_seen: row.get(4)?,
                    last_seen: row.get(5)?,
                    occurrences: row.get(6)?,
                    secrets_found: row.get::<_, Option<String>>(7)?.map(|s| {
                        serde_json::from_str(&s).unwrap_or_default()
                    }),
                    status: EntryStatus::from(row.get::<_, String>(8)?.as_str()),
                })
            },
        );

        match existing {
            Ok(mut entry) => {
                // Duplicate — increment occurrence count
                self.conn.execute(
                    "UPDATE training_entries SET occurrences = occurrences + 1, last_seen = CURRENT_TIMESTAMP WHERE content_hash = ?1",
                    params![hash],
                )?;
                entry.occurrences += 1;
                Ok(ProcessResult::Duplicate(entry))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // New entry — insert into registry
                let secrets_json = secrets_found
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default());
                self.conn.execute(
                    "INSERT INTO training_entries (content_hash, original_path, first_seen, last_seen, occurrences, secrets_found, status)
                     VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1, ?3, 'pending')",
                    params![hash, original_path, secrets_json],
                )?;

                let entry = TrainingEntry {
                    id: self.conn.last_insert_rowid(),
                    content_hash: hash,
                    original_path,
                    processed_path: None,
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                    occurrences: 1,
                    secrets_found,
                    status: EntryStatus::Pending,
                };
                Ok(ProcessResult::New(entry))
            }
            Err(e) => Err(DedupError::Sqlite(e)),
        }
    }

    /// Mark an entry as clean and set its processed path.
    pub fn mark_clean(
        &self,
        content_hash: impl AsRef<str>,
        processed_path: impl AsRef<str>,
    ) -> Result<(), DedupError> {
        let hash = content_hash.as_ref();
        let path = processed_path.as_ref();

        self.conn.execute(
            "UPDATE training_entries SET status = 'clean', processed_path = ?2 WHERE content_hash = ?1",
            params![hash, path],
        )?;
        Ok(())
    }

    /// Query all entries with a given status.
    pub fn query_by_status(&self, status: EntryStatus) -> Result<Vec<TrainingEntry>, DedupError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content_hash, original_path, processed_path, first_seen, last_seen, occurrences, secrets_found, status
             FROM training_entries WHERE status = ?1 ORDER BY last_seen DESC"
        )?;

        let entries: Vec<TrainingEntry> = stmt
            .query_map(params![status.to_string()], |row| {
                let secrets_str: Option<String> = row.get(7)?;
                let status_str: String = row.get(8)?;
                Ok(TrainingEntry {
                    id: row.get(0)?,
                    content_hash: row.get(1)?,
                    original_path: row.get(2)?,
                    processed_path: row.get(3)?,
                    first_seen: row.get(4)?,
                    last_seen: row.get(5)?,
                    occurrences: row.get(6)?,
                    secrets_found: secrets_str.map(|s| {
                        serde_json::from_str(&s).unwrap_or_default()
                    }),
                    status: EntryStatus::from(status_str.as_str()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get statistics about the registry.
    pub fn stats(&self) -> Result<HashMap<String, i64>, DedupError> {
        let mut stats = HashMap::new();

        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM training_entries",
            [],
            |row| row.get(0),
        )?;
        stats.insert("total".to_string(), total);

        for status in [
            EntryStatus::Pending,
            EntryStatus::Clean,
            EntryStatus::Duplicate,
            EntryStatus::Error,
        ] {
            let count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM training_entries WHERE status = ?1",
                params![status.to_string()],
                |row| row.get(0),
            )?;
            stats.insert(status.to_string(), count);
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_engine() -> (DedupEngine, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("dedup.db");
        let engine = DedupEngine::new(&path).unwrap();
        (engine, dir)
    }

    #[test]
    fn computes_consistent_hashes() {
        let hash1 = DedupEngine::compute_hash("Hello World");
        let hash2 = DedupEngine::compute_hash("hello world");
        // Case-folding should produce same hash
        assert_eq!(hash1, hash2);

        let hash3 = DedupEngine::compute_hash("Different text");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn normalizes_whitespace() {
        let hash1 = DedupEngine::compute_hash("Hello   World");
        let hash2 = DedupEngine::compute_hash("Hello World");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn detects_duplicates() {
        let (engine, _dir) = make_engine();

        let result1 = engine.process_file("file1.txt", "Same content here", None).unwrap();
        assert!(matches!(result1, ProcessResult::New(_)));

        let result2 = engine.process_file("file2.txt", "Same content here", None).unwrap();
        assert!(matches!(result2, ProcessResult::Duplicate(_)));

        if let ProcessResult::Duplicate(entry) = result2 {
            assert_eq!(entry.occurrences, 2);
        }
    }

    #[test]
    fn tracks_unique_entries() {
        let (engine, _dir) = make_engine();

        engine.process_file("file1.txt", "Content A", None).unwrap();
        engine.process_file("file2.txt", "Content B", None).unwrap();

        let stats = engine.stats().unwrap();
        assert_eq!(stats["total"], 2);
    }

    #[test]
    fn marks_entries_clean() {
        let (engine, _dir) = make_engine();

        let result = engine.process_file("file1.txt", "Test content", None).unwrap();
        if let ProcessResult::New(entry) = result {
            engine.mark_clean(&entry.content_hash, "/processed/path.md").unwrap();

            let clean_entries = engine.query_by_status(EntryStatus::Clean).unwrap();
            assert_eq!(clean_entries.len(), 1);
            assert_eq!(clean_entries[0].processed_path, Some("/processed/path.md".to_string()));
        }
    }
}
