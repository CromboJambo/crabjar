//! Approval Queue Service
//!
//! Provides the core approval workflow functionality including queue management,
//! impact assessment integration, and audit trail.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents the status of a change in the approval queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    Pending,
    Approved,
    Rejected,
    Withdrawn,
}

impl std::fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeStatus::Pending => write!(f, "pending"),
            ChangeStatus::Approved => write!(f, "approved"),
            ChangeStatus::Rejected => write!(f, "rejected"),
            ChangeStatus::Withdrawn => write!(f, "withdrawn"),
        }
    }
}

/// Represents a single change awaiting or undergoing approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub submitter_id: String,
    pub submitted_at: i64,
    pub status: ChangeStatus,
    pub affected_projects: Vec<String>,
    pub required_approvals: Vec<String>,
    pub min_approvals_required: u8,
    pub approvals_received: u8,
    pub rejection_reason: Option<String>,
}

/// The approval queue service.
pub struct ApprovalQueueService {
    changes: HashMap<String, ChangeRequest>,
    audit_trail: Vec<AuditEntry>,
}

impl ApprovalQueueService {
    /// Create a new approval queue service.
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
            audit_trail: Vec::new(),
        }
    }

    /// Add a change to the approval queue.
    pub fn add_change(&mut self, change: ChangeRequest) -> String {
        let id = change.id.clone();
        self.changes.insert(id.clone(), change);
        self.audit_trail.push(AuditEntry {
            change_id: id.clone(),
            action: "submitted".to_string(),
            actor: "system".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        });
        id
    }

    /// Get all pending approvals.
    pub fn get_pending(&self) -> Vec<&ChangeRequest> {
        self.changes
            .values()
            .filter(|c| c.status == ChangeStatus::Pending)
            .collect()
    }

    /// Approve a specific change by ID.
    pub fn approve_change(&mut self, change_id: &str, approver_id: &str) -> bool {
        if let Some(change) = self.changes.get_mut(change_id) {
            if change.status == ChangeStatus::Pending {
                change.approvals_received += 1;
                if change.approvals_received >= change.min_approvals_required {
                    change.status = ChangeStatus::Approved;
                }
                self.audit_trail.push(AuditEntry {
                    change_id: change_id.to_string(),
                    action: "approved".to_string(),
                    actor: approver_id.to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                });
                return true;
            }
        }
        false
    }

    /// Reject a specific change by ID.
    pub fn reject_change(&mut self, change_id: &str, approver_id: &str, reason: &str) -> bool {
        if let Some(change) = self.changes.get_mut(change_id) {
            if change.status == ChangeStatus::Pending {
                change.status = ChangeStatus::Rejected;
                change.rejection_reason = Some(reason.to_string());
                self.audit_trail.push(AuditEntry {
                    change_id: change_id.to_string(),
                    action: "rejected".to_string(),
                    actor: approver_id.to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                });
                return true;
            }
        }
        false
    }

    /// Get the audit trail for a specific change.
    pub fn get_audit_trail(&self, change_id: &str) -> Vec<&AuditEntry> {
        self.audit_trail
            .iter()
            .filter(|e| e.change_id == change_id)
            .collect()
    }
}

/// An audit entry for tracking approval decisions.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub change_id: String,
    pub action: String,
    pub actor: String,
    pub timestamp: i64,
}
