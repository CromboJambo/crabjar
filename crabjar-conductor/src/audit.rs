//! Audit logging infrastructure using mirror-log for transparent event tracking.
//!
//! This module provides an append-only event log for crabjar conductor operations,
//! enabling evidence-based promotion and knowledge compression by recording task
//! completions with flattened context (useful for evaluation, not raw sensitive data).

use anyhow::{Context, Result};
use chrono::Utc;
use mirror_log::{LogEntry, LogStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Audit event types for conductor operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    TaskCompleted,
    TaskFailed,
    WorkerPromoted,
    WorkerDemoted,
    GoalDecomposed,
    RouteDecision,
}

/// Flattened context for an audit event — useful for evaluation without raw sensitive data.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    store: Arc<Mutex<LogStore>>,
}

impl ConductorAuditLogger {
    /// Create a new audit logger with the specified database path.
    pub fn new(db_path: &str) -> Result<Self> {
        let store = LogStore::open(db_path).context("Failed to open audit log store")?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }

    /// Log an audit event with flattened context.
    pub async fn log_event(&self, event_type: AuditEventType, context: AuditContext) -> Result<Uuid> {
        let store = self.store.lock().await;

        let entry = LogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: "crabjar-conductor".to_string(),
            event_type: format!("{:?}", event_type).to_lowercase(),
            payload: serde_json::to_string(&context)?,
            tags: vec!["audit".to_string()],
        };

        let entry_id = store.append(entry)?;
        Ok(entry_id)
    }

    /// Query audit events by task ID.
    pub async fn query_by_task(&self, task_id: &str) -> Result<Vec<LogEntry>> {
        let store = self.store.lock().await;
        // Filter entries by task_id in payload (simplified — real implementation would use tags or structured queries)
        Ok(vec![]) // Placeholder — mirror-log query API needs investigation
    }

    /// Query audit events by worker ID.
    pub async fn query_by_worker(&self, worker_id: &str) -> Result<Vec<LogEntry>> {
        let store = self.store.lock().await;
        Ok(vec![]) // Placeholder
    }

    /// Get recent audit events (for debugging/monitoring).
    pub async fn recent_events(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let store = self.store.lock().await;
        Ok(store.recent(limit))
    }
}

/// Log a task completion event.
pub async fn log_task_completion(
    logger: &ConductorAuditLogger,
    task_id: String,
    worker_id: String,
    outcome: String,
    duration_ms: u64,
) -> Result<Uuid> {
    let context = AuditContext {
        task_id: Some(task_id),
        worker_id: Some(worker_id.clone()),
        outcome,
        duration_ms: Some(duration_ms),
        metadata: serde_json::json!({}),
    };

    logger.log_event(AuditEventType::TaskCompleted, context).await
}

/// Log a route decision event.
pub async fn log_route_decision(
    logger: &ConductorAuditLogger,
    task_id: String,
    worker_id: String,
    reason: String,
) -> Result<Uuid> {
    let context = AuditContext {
        task_id: Some(task_id),
        worker_id: Some(worker_id.clone()),
        outcome: format!("routed: {}", reason),
        duration_ms: None,
        metadata: serde_json::json!({}),
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
        assert!(logger.store.lock().await.is_open());
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

        assert_ne!(entry_id, Uuid::nil());
        Ok(())
    }
}
