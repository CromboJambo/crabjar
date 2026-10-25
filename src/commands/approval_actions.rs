//! Approve/Reject Commands
//!
//! Provides CLI commands for approving and rejecting changes with validation.

use crate::commands::approval_service::{ApprovalQueueService, ChangeStatus};
use std::collections::HashMap;

/// Validate that a change exists and is in pending state.
fn validate_change(service: &ApprovalQueueService, change_id: &str) -> bool {
    let pending = service.get_pending();
    pending.iter().any(|c| c.id == change_id)
}

/// Approve a specific change by ID with confirmation.
pub fn approve_change(service: &mut ApprovalQueueService, change_id: &str) -> String {
    if !validate_change(service, change_id) {
        return format!("Error: Change {} not found or not in pending state.", change_id);
    }

    // In a real CLI, this would prompt for confirmation
    // For now, we'll just proceed with the approval
    if service.approve_change(change_id, "user") {
        format!("Change {} approved successfully!", change_id)
    } else {
        format!("Failed to approve change {}.", change_id)
    }
}

/// Reject a change by ID with required reason.
pub fn reject_change(service: &mut ApprovalQueueService, change_id: &str, reason: &str) -> String {
    if !validate_change(service, change_id) {
        return format!("Error: Change {} not found or not in pending state.", change_id);
    }

    if service.reject_change(change_id, "user", reason) {
        format!("Change {} rejected successfully with reason: {}", change_id, reason)
    } else {
        format!("Failed to reject change {}.", change_id)
    }
}
