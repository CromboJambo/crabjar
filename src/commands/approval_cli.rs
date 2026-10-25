//! Approval CLI Interface
//!
//! Provides user-facing commands for interacting with the approval workflow.

use crate::commands::approval_service::ApprovalQueueService;
use std::collections::HashMap;

/// Represents a CLI command for the approval interface.
pub enum ApprovalCommand {
    /// List pending approvals.
    ListPending,
    /// Approve a specific change by ID.
    Approve { change_id: String },
    /// Reject a change with reason.
    Reject { change_id: String, reason: String },
    /// View approval history/audit trail.
    History { change_id: Option<String> },
}

/// Approval CLI handler that processes commands.
pub struct ApprovalCliHandler {
    service: ApprovalQueueService,
}

impl ApprovalCliHandler {
    /// Create a new approval CLI handler.
    pub fn new() -> Self {
        Self {
            service: ApprovalQueueService::new(),
        }
    }

    /// Process an approval command and return the result.
    pub fn process_command(&mut self, command: ApprovalCommand) -> String {
        match command {
            ApprovalCommand::ListPending => self.list_pending_approvals(),
            ApprovalCommand::Approve { change_id } => self.approve_change(change_id),
            ApprovalCommand::Reject { change_id, reason } => self.reject_change(change_id, reason),
            ApprovalCommand::History { change_id } => self.view_history(change_id),
        }
    }

    /// List all pending approvals.
    fn list_pending_approvals(&self) -> String {
        let pending = self.service.get_pending();
        if pending.is_empty() {
            return "No pending approvals.".to_string();
        }

        let mut output = format!("Pending Approvals ({}):\n", pending.len());
        for change in pending {
            output.push_str(&format!(
                "  [{}] {} - submitted by {}\n",
                change.id, change.title, change.submitter_id
            ));
        }
        output
    }

    /// Approve a specific change by ID.
    fn approve_change(&mut self, change_id: String) -> String {
        if self.service.approve_change(&change_id, "user") {
            format!("Change {} approved.", change_id)
        } else {
            format!("Failed to approve change {}.", change_id)
        }
    }

    /// Reject a change with reason.
    fn reject_change(&mut self, change_id: String, reason: String) -> String {
        if self.service.reject_change(&change_id, "user", &reason) {
            format!("Change {} rejected.", change_id)
        } else {
            format!("Failed to reject change {}.", change_id)
        }
    }

    /// View approval history/audit trail.
    fn view_history(&self, change_id: Option<String>) -> String {
        match change_id {
            Some(id) => {
                let audit = self.service.get_audit_trail(&id);
                if audit.is_empty() {
                    return format!("No history for change {}.", id);
                }

                let mut output = format!("History for change {}:\n", id);
                for entry in audit {
                    output.push_str(&format!(
                        "  [{}] {} by {}\n",
                        entry.timestamp, entry.action, entry.actor
                    ));
                }
                output
            }
            None => {
                // Show all history (simplified)
                "Showing all approval history...".to_string()
            }
        }
    }
}
