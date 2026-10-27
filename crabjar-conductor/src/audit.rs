//! Audit logging infrastructure using mirror-log for transparent event tracking.
//!
//! This module provides an append-only event log for crabjar conductor operations,
//! enabling evidence-based promotion and knowledge compression by recording task
//! completions with flattened context (useful for evaluation, not raw sensitive data).

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Audit event types for conductor operations.
#[derive(Debug, Clone)]
pub enum AuditEventType {
    TaskCompleted,
    TaskFailed,
    WorkerPromoted,
    WorkerDemoted,
    GoalDecomposed,
    RouteDecision,
}

/// Flattened context for an audit event — useful for evaluation without raw sensitive data.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Task or operation identifier.
    pub task_id: Option<String>,
    /// Worker identifier involved.
    pub worker_id: Option<String>,
    /// Outcome summary (success/failure/error message).
    pub outcome: String,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Additional structured metadata.
    pub metadata: serde_json::Value,
}

/// Audit logger that wraps mirror-log for conductor operations.
pub struct ConductorAuditLogger {
    conn: Arc<Mutex<Connection>>,
}

impl ConductorAuditLogger {
    /// Create a new audit logger with the specified database path.
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open audit log database")?;
        
        // Initialize mirror-log tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (\n\
             id TEXT PRIMARY KEY,\n\
             timestamp INTEGER NOT NULL,\n\
             source TEXT NOT NULL,\n\
             content TEXT NOT NULL,\n\
             meta TEXT,\n\
             ingested_at INTEGER NOT NULL,\n\
             content_hash TEXT\n\
             );\n\
             CREATE TABLE IF NOT EXISTS active_events (\n\
             id TEXT PRIMARY KEY REFERENCES events(id),\n\
             timestamp INTEGER NOT NULL,\n\
             source TEXT NOT NULL,\n\
             content TEXT NOT NULL,\n\
             meta TEXT,\n\
             ingested_at INTEGER NOT NULL,\n\
             content_hash TEXT\n\
             );",
        )
        .context("Failed to initialize audit log tables")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Log an audit event with flattened context.
    pub async fn log_event(&self, event_type: AuditEventType, context: AuditContext) -> Result<String> {
        let conn = self.conn.lock().await;

        let payload = json!({
            "event_type": format!("{:?}", event_type).to_lowercase(),
            "timestamp": Utc::now().to_rfc3339(),
            "source": "crabjar-conductor",
            "task_id": context.task_id,
            "worker_id": context.worker_id,
            "outcome": context.outcome,
            "duration_ms": context.duration_ms,
            "metadata": context.metadata,
        });

        let text = payload.to_string();

        mirror_log::append(&conn, "crabjar-conductor", &text, None)
            .context("Failed to append audit event")
    }

    /// Query audit events by task ID.
    pub async fn query_by_task(&self, task_id: &str) -> Result<Vec<mirror_log::Event>> {
        let conn = self.conn.lock().await;
        mirror_log::view::search(&conn, task_id).context("Failed to search audit events")
    }

    /// Query audit events by worker ID.
    pub async fn query_by_worker(&self, worker_id: &str) -> Result<Vec<mirror_log::Event>> {
        let conn = self.conn.lock().await;
        mirror_log::view::search(&conn, worker_id).context("Failed to search audit events")
    }

    /// Get recent audit events (for debugging/monitoring).
    pub async fn recent_events(&self, limit: usize) -> Result<Vec<mirror_log::Event>> {
        let conn = self.conn.lock().await;
        mirror_log::view::recent(&conn, limit as i64).context("Failed to get recent audit events")
    }
}

/// Log a task completion event.
pub async fn log_task_completion(
    logger: &ConductorAuditLogger,
    task_id: String,
    worker_id: String,
    outcome: String,
    duration_ms: u64,
) -> Result<String> {
    let context = AuditContext {
        task_id: Some(task_id),
        worker_id: Some(worker_id.clone()),
        outcome,
        duration_ms: Some(duration_ms),
        metadata: json!({}),
    };

    logger.log_event(AuditEventType::TaskCompleted, context).await
}

/// Log a route decision event.
pub async fn log_route_decision(
    logger: &ConductorAuditLogger,
    task_id: String,
    worker_id: String,
    reason: String,
) -> Result<String> {
    let context = AuditContext {
        task_id: Some(task_id),
        worker_id: Some(worker_id.clone()),
        outcome: format!("routed: {}", reason),
        duration_ms: None,
        metadata: json!({}),
    };

    logger.log_event(AuditEventType::RouteDecision, context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_audit_logger_creation() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("audit.db").to_str().unwrap();
        let logger = ConductorAuditLogger::new(db_path)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_log_task_completion() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("audit.db").to_str().unwrap();
        let logger = ConductorAuditLogger::new(db_path)?;

        let entry_id = log_task_completion(
            &logger,
            "task-123".to_string(),
            "worker-456".to_string(),
            "success".to_string(),
            1000,
        )
        .await?;

        assert_ne!(entry_id, "");
        Ok(())
    }
}
