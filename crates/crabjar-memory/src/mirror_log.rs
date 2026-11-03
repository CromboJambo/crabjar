//! Optional mirror-log integration for memory operations.
//!
//! When enabled, every memory operation (add/patch/remove) is logged to
//! mirror-log with provenance metadata, enabling attention tracking and
//! evidence-based promotion of important memories.

use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "mirror-log")]
use rusqlite::Connection;

/// Audit event types for memory operations.
#[derive(Debug, Clone)]
pub enum MemoryEventType {
    EntryAdded,
    EntryPatched,
    EntryRemoved,
    BatchApplied,
}

impl std::fmt::Display for MemoryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryEventType::EntryAdded => write!(f, "memory.entry_added"),
            MemoryEventType::EntryPatched => write!(f, "memory.entry_patched"),
            MemoryEventType::EntryRemoved => write!(f, "memory.entry_removed"),
            MemoryEventType::BatchApplied => write!(f, "memory.batch_applied"),
        }
    }
}

/// Audit context for a memory operation.
#[derive(Debug, Clone)]
pub struct MemoryAuditContext {
    /// Target store ("memory" or "user").
    pub target: String,
    /// Operation type.
    pub event_type: MemoryEventType,
    /// Entry content (or partial match for patch/remove).
    pub content: Option<String>,
    /// Old text for patch operations.
    pub old_text: Option<String>,
    /// Number of operations in batch (for batch operations).
    pub operation_count: Option<usize>,
}

/// Memory audit logger that integrates with mirror-log.
pub struct MemoryAuditLogger {
    #[cfg(feature = "mirror-log")]
    conn: Arc<Mutex<Connection>>,
    #[cfg(not(feature = "mirror-log"))]
    _placeholder: (),
}

impl MemoryAuditLogger {
    /// Create a new memory audit logger with the specified mirror-log database path.
    #[cfg(feature = "mirror-log")]
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        // Initialize mirror-log tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
             id TEXT PRIMARY KEY,
             timestamp INTEGER NOT NULL,
             source TEXT NOT NULL,
             content TEXT NOT NULL,
             meta TEXT,
             ingested_at INTEGER NOT NULL,
             content_hash TEXT
             );"
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a no-op logger when mirror-log feature is not enabled.
    #[cfg(not(feature = "mirror-log"))]
    pub fn new(_db_path: &str) -> anyhow::Result<Self> {
        Ok(Self { _placeholder: () })
    }

    /// Log a memory operation event.
    pub async fn log_event(&self, context: MemoryAuditContext) -> anyhow::Result<String> {
        #[cfg(feature = "mirror-log")]
        {
            let conn = self.conn.lock().await;
            let payload = json!({
                "event_type": context.event_type.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "crabjar-memory",
                "target": context.target,
                "content": context.content,
                "old_text": context.old_text,
                "operation_count": context.operation_count,
            });
            let text = payload.to_string();
            Ok(mirror_log::append(&conn, "crabjar-memory", &text, None)?)
        }
        #[cfg(not(feature = "mirror-log"))]
        {
            // No-op when mirror-log feature is not enabled
            Ok("noop".to_string())
        }
    }

    /// Query memory events by target store.
    pub async fn query_by_target(&self, target: &str) -> anyhow::Result<Vec<mirror_log::Event>> {
        #[cfg(feature = "mirror-log")]
        {
            let conn = self.conn.lock().await;
            Ok(mirror_log::view::search(&conn, target)?)
        }
        #[cfg(not(feature = "mirror-log"))]
        {
            Ok(vec![])
        }
    }

    /// Get recent memory events.
    pub async fn recent_events(&self, limit: usize) -> anyhow::Result<Vec<mirror_log::Event>> {
        #[cfg(feature = "mirror-log")]
        {
            let conn = self.conn.lock().await;
            Ok(mirror_log::view::recent(&conn, limit as i64)?)
        }
        #[cfg(not(feature = "mirror-log"))]
        {
            Ok(vec![])
        }
    }
}

/// Log a memory entry addition.
pub async fn log_entry_added(
    logger: &MemoryAuditLogger,
    target: &str,
    content: &str,
) -> anyhow::Result<String> {
    let context = MemoryAuditContext {
        target: target.to_string(),
        event_type: MemoryEventType::EntryAdded,
        content: Some(content.to_string()),
        old_text: None,
        operation_count: None,
    };
    logger.log_event(context).await
}

/// Log a memory entry patch.
pub async fn log_entry_patched(
    logger: &MemoryAuditLogger,
    target: &str,
    old_text: &str,
    new_content: &str,
) -> anyhow::Result<String> {
    let context = MemoryAuditContext {
        target: target.to_string(),
        event_type: MemoryEventType::EntryPatched,
        content: Some(new_content.to_string()),
        old_text: Some(old_text.to_string()),
        operation_count: None,
    };
    logger.log_event(context).await
}

/// Log a memory entry removal.
pub async fn log_entry_removed(
    logger: &MemoryAuditLogger,
    target: &str,
    old_text: &str,
) -> anyhow::Result<String> {
    let context = MemoryAuditContext {
        target: target.to_string(),
        event_type: MemoryEventType::EntryRemoved,
        content: None,
        old_text: Some(old_text.to_string()),
        operation_count: None,
    };
    logger.log_event(context).await
}

/// Log a batch of memory operations.
pub async fn log_batch_applied(
    logger: &MemoryAuditLogger,
    target: &str,
    operation_count: usize,
) -> anyhow::Result<String> {
    let context = MemoryAuditContext {
        target: target.to_string(),
        event_type: MemoryEventType::BatchApplied,
        content: None,
        old_text: None,
        operation_count: Some(operation_count),
    };
    logger.log_event(context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memory_audit_logger_creation() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let binding = dir.path().join("memory-audit.db");
        let db_path = binding.to_str().unwrap();
        let logger = MemoryAuditLogger::new(db_path)?;
        drop(logger);
        Ok(())
    }

    #[tokio::test]
    async fn test_log_entry_added() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let binding = dir.path().join("memory-audit.db");
        let db_path = binding.to_str().unwrap();
        let logger = MemoryAuditLogger::new(db_path)?;

        let entry_id = log_entry_added(&logger, "memory", "Test memory entry").await?;
        assert_ne!(entry_id, "");
        Ok(())
    }
}