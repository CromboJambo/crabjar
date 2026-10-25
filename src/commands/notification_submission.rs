//! Submission Notifications
//!
//! Notifies approvers when changes are submitted for approval.

use crate::commands::notification_infrastructure::{Notification, NotificationChannel, NotificationService};
use std::collections::HashMap;

/// Send submission notifications to all required approvers.
pub fn notify_approvers(
    service: &mut NotificationService,
    change_id: &str,
    title: &str,
    summary: &str,
    impact_level: &str,
    approvers: Vec<String>,
) -> Vec<String> {
    let mut sent_ids = Vec::new();

    for approver in approvers {
        let notification = Notification::new(
            approver.clone(),
            format!("Change submitted for approval: {}", title),
            format!(
                "Change ID: {}\nTitle: {}\nSummary: {}\nImpact Level: {}\n\nPlease review and approve or reject.",
                change_id, title, summary, impact_level
            ),
            NotificationChannel::Email,
        );

        if service.send_notification(notification) {
            sent_ids.push(approver);
        }
    }

    sent_ids
}
