//! Decision Notifications
//!
//! Notifies submitters when their changes are approved or rejected.

use crate::commands::notification_infrastructure::{Notification, NotificationChannel, NotificationService};

/// Send decision notification to submitter.
pub fn notify_submitter(
    service: &mut NotificationService,
    submitter: String,
    change_id: &str,
    title: &str,
    decision: &str,
    approver: &str,
    reason: Option<&str>,
) -> bool {
    let subject = match decision {
        "approved" => format!("Change approved: {}", title),
        "rejected" => format!("Change rejected: {}", title),
        _ => format!("Decision on change: {}", title),
    };

    let body = match decision {
        "approved" => format!(
            "Your change '{}' has been approved by {}.",
            title, approver
        ),
        "rejected" => format!(
            "Your change '{}' has been rejected by {}. Reason: {}",
            title,
            approver,
            reason.unwrap_or("No reason provided")
        ),
        _ => format!("Decision made on your change '{}'.", title),
    };

    let notification = Notification::new(submitter, subject, body, NotificationChannel::Email);
    service.send_notification(notification)
}
