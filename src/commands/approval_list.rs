//! List Pending Approvals Command
//!
//! Displays all pending approval requests with impact levels in a formatted table.

use crate::commands::approval_service::{ApprovalQueueService, ChangeStatus};
use std::collections::HashMap;

/// Represents an approval request for display purposes.
pub struct ApprovalDisplay {
    pub id: String,
    pub title: String,
    pub submitter: String,
    pub impact_level: String,
    pub submitted_at: String,
    pub status: String,
}

impl ApprovalDisplay {
    /// Format the approval for display with color-coded impact level.
    pub fn format(&self) -> String {
        let impact_color = match self.impact_level.as_str() {
            "low" => "\x1b[32m", // green
            "medium" => "\x1b[33m", // yellow
            "high" => "\x1b[31m", // red
            "critical" => "\x1b[91m", // bright red
            _ => "",
        };
        let reset = "\x1b[0m";

        format!(
            "{} [{}] {} - submitted by {} at {}",
            impact_color,
            self.id,
            self.title,
            self.submitter,
            self.submitted_at,
            reset
        )
    }
}

/// List all pending approvals with optional filters.
pub fn list_pending_approvals(
    service: &ApprovalQueueService,
    owner_filter: Option<&str>,
    app_filter: Option<&str>,
) -> Vec<ApprovalDisplay> {
    let pending = service.get_pending();
    let mut results = Vec::new();

    for change in pending {
        // Apply filters
        if let Some(owner) = owner_filter {
            if change.submitter_id != owner {
                continue;
            }
        }

        if let Some(app) = app_filter {
            if !change.affected_projects.iter().any(|p| p == app) {
                continue;
            }
        }

        results.push(ApprovalDisplay {
            id: change.id.clone(),
            title: change.title.clone(),
            submitter: change.submitter_id.clone(),
            impact_level: "medium".to_string(), // Would be determined by impact assessment
            submitted_at: format!("{}", change.submitted_at),
            status: change.status.to_string(),
        });
    }

    results
}

/// Display the list of pending approvals.
pub fn display_approval_list(approvals: &[ApprovalDisplay]) {
    if approvals.is_empty() {
        println!("No pending approvals.");
        return;
    }

    println!("Pending Approvals ({}):", approvals.len());
    for approval in approvals {
        println!("{}", approval.format());
    }
}
