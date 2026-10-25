//! Approval History Command
//!
//! Provides CLI commands for viewing approval history and audit trails.

use crate::commands::approval_service::{ApprovalQueueService, AuditEntry};

/// Display approval history for a specific change or all changes.
pub fn display_approval_history(
    service: &ApprovalQueueService,
    change_id: Option<&str>,
) {
    match change_id {
        Some(id) => {
            let audit = service.get_audit_trail(id);
            if audit.is_empty() {
                println!("No history found for change {}.", id);
                return;
            }

            println!("Approval history for change {}:", id);
            print_audit_entries(audit);
        }
        None => {
            // Show all history (simplified - would need to aggregate from service)
            println!("Showing all approval history...");
            // In production, this would fetch all audit entries from the service
        }
    }
}

/// Print a list of audit entries in a readable timeline format.
fn print_audit_entries(entries: Vec<&AuditEntry>) {
    for entry in entries {
        println!(
            "[{}] {} by {}",
            entry.timestamp, entry.action, entry.actor
        );
    }
}
