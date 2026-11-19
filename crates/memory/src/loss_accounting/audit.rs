//! Compression audit log — every compression operation is logged.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::models::{LossProfileId, SourceRef, Timestamp};

/// Reference to an approver (supervisor, policy engine, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproverRef {
    pub id: String,
    pub role: String,
}

impl ApproverRef {
    pub fn new(id: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: role.into(),
        }
    }
}

/// A single entry in the compression audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionAuditEntry {
    pub timestamp: Timestamp,
    pub worker_id: String,
    pub source_ref: SourceRef,
    pub profile_id: LossProfileId,
    pub decision_rationale: String,
    pub approved_by: Option<ApproverRef>,
}

impl CompressionAuditEntry {
    pub fn new(
        worker_id: impl Into<String>,
        source_ref: SourceRef,
        profile_id: LossProfileId,
        rationale: impl Into<String>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            timestamp: now,
            worker_id: worker_id.into(),
            source_ref,
            profile_id,
            decision_rationale: rationale.into(),
            approved_by: None,
        }
    }

    pub fn with_approval(mut self, approver: ApproverRef) -> Self {
        self.approved_by = Some(approver);
        self
    }
}

/// In-memory compression audit log.
#[derive(Debug, Default)]
pub struct CompressionAuditLog {
    entries: Arc<Mutex<Vec<CompressionAuditEntry>>>,
}

impl CompressionAuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a compression operation.
    pub fn record(&self, entry: CompressionAuditEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    /// Get all entries for a specific worker.
    pub fn entries_for_worker(&self, worker_id: &str) -> Vec<CompressionAuditEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.worker_id == worker_id)
            .cloned()
            .collect()
    }

    /// Get all entries for a specific source.
    pub fn entries_for_source(&self, source_id: &str) -> Vec<CompressionAuditEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.source_ref.id == source_id)
            .cloned()
            .collect()
    }

    /// Get all entries.
    pub fn all_entries(&self) -> Vec<CompressionAuditEntry> {
        self.entries.lock().unwrap().clone()
    }

    /// Count of entries.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
