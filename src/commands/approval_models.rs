//! Approval Queue Data Models
//!
//! Defines the data structures for the approval workflow system.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Timestamp type for storage operations.
pub type Timestamp = u64;

/// Impact levels for changes requiring approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// Low-impact change that can proceed automatically.
    Low,
    /// Medium-impact change that should be reviewed but not blocked.
    Medium,
    /// High-impact change that requires explicit approval.
    High,
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactLevel::Low => write!(f, "low"),
            ImpactLevel::Medium => write!(f, "medium"),
            ImpactLevel::High => write!(f, "high"),
        }
    }
}

/// Status of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Request is pending approval.
    Pending,
    /// Request has been approved.
    Approved,
    /// Request has been rejected.
    Rejected,
    /// Request was withdrawn before decision.
    Withdrawn,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "pending"),
            ApprovalStatus::Approved => write!(f, "approved"),
            ApprovalStatus::Rejected => write!(f, "rejected"),
            ApprovalStatus::Withdrawn => write!(f, "withdrawn"),
        }
    }
}

/// A change that may require approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: String,
    pub description: String,
    pub impact_level: ImpactLevel,
    pub affected_services: Vec<String>,
    pub affected_users: usize,
    pub change_type: String,
    pub created_at: Timestamp,
}

/// An approval request for a specific change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub change_id: String,
    pub approver_id: String,
    pub status: ApprovalStatus,
    pub created_at: Timestamp,
    pub decided_at: Option<Timestamp>,
}

/// The approval queue manager.
pub struct ApprovalQueue {
    requests: Vec<ApprovalRequest>,
}

impl ApprovalQueue {
    /// Create a new empty approval queue.
    pub fn new() -> Self {
        Self { requests: Vec::new() }
    }

    /// Add an approval request to the queue.
    pub fn add_request(&mut self, request: ApprovalRequest) {
        self.requests.push(request);
    }

    /// Get all pending approval requests.
    pub fn get_pending(&self) -> Vec<&ApprovalRequest> {
        self.requests
            .iter()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .collect()
    }

    /// Approve a specific request by ID.
    pub fn approve(&mut self, request_id: &str) -> bool {
        if let Some(request) = self.requests.iter_mut().find(|r| r.id == request_id) {
            request.status = ApprovalStatus::Approved;
            return true;
        }
        false
    }

    /// Reject a specific request by ID.
    pub fn reject(&mut self, request_id: &str) -> bool {
        if let Some(request) = self.requests.iter_mut().find(|r| r.id == request_id) {
            request.status = ApprovalStatus::Rejected;
            return true;
        }
        false
    }
}
