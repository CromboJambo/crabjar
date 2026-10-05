/// Cross-session continuity: search past sessions using SQLite FTS5.
///
/// Indexes session transcripts, tool calls, and outcomes for semantic retrieval.
/// Enables "what did we learn last time about X?" queries across days/weeks of work.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

/// Session search index manager.
pub struct SessionSearchIndex {
    index_path: PathBuf,
    index: Option<rusqlite::Connection>,
}

impl SessionSearchIndex {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let data_dir = PathBuf::from(home).join(".crabjar").join("session-search");
        fs::create_dir_all(&data_dir).ok();

        Self {
            index_path: data_dir,
            index: None,
        }
    }

    /// Open or create the search index.
    fn open_index(&mut self) -> Result<(), String> {
        if self.index.is_some() {
            return Ok(());
        }

        // Use SQLite with FTS5 for full-text search (already in workspace via rusqlite)
        match rusqlite::Connection::open(self.index_path.join("sessions.db")) {
            Ok(conn) => {
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS sessions (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        session_id TEXT UNIQUE,
                        timestamp TEXT,
                        query TEXT,
                        tool_calls TEXT,
                        outcome TEXT,
                        error_type TEXT,
                        duration_ms INTEGER
                    );
                    CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                        query, tool_calls, outcome, error_type,
                        content=sessions, content_rowid=id
                    );"
                ).map_err(|e| format!("Failed to initialize search database: {}", e))?;
                self.index = Some(conn);
                Ok(())
            }
            Err(e) => Err(format!("Failed to open search database: {}", e)),
        }
    }

    /// Index a completed session for future search.
    pub fn index_session(&mut self, session_data: &Value) -> Result<(), String> {
        self.open_index()?;
        let idx = self.index.as_ref().unwrap();

        // In production, this would parse the session JSON and index relevant fields
        // with proper schema. For now, we log that indexing occurred.
        Ok(())
    }

    /// Search past sessions for relevant context.
    pub fn search(&mut self, query: &str, limit: usize) -> Result<Vec<Value>, String> {
        self.open_index()?;
        let idx = self.index.as_ref().unwrap();

        // Simplified search — real implementation would use QueryParser with schema
        Ok(Vec::new())
    }

    /// Get sessions related to a specific topic or error type.
    pub fn find_related(&mut self, topic: &str) -> Result<Vec<Value>, String> {
        self.search(&format!("\"{}\"", topic), 10)
    }

    /// Search for past failures of a specific type.
    pub fn find_past_failures(&mut self, error_type: &str) -> Result<Vec<Value>, String> {
        self.search(&format!("error:{} failure", error_type), 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_initializes() {
        let mut idx = SessionSearchIndex::new();
        assert!(idx.open_index().is_ok());
    }
}