//! Comprehensive Integration Tests for Crabjar Notification System
//!
//! Validates all notification components work together correctly.

#[cfg(test)]
mod tests {
    use crate::commands::notification_infrastructure::{
        Notification, NotificationChannel, NotificationService, EmailNotifier, SlackNotifier, CliNotifier, Notifier
    };
    use crate::commands::notification_submission::notify_approvers;
    use crate::commands::notification_decision::notify_submitter;
    use crate::commands::notification_preferences::{NotificationPreferences, PreferenceService};

    // Infrastructure Tests
    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            "user@example.com".to_string(),
            "Test Subject".to_string(),
            "Test Body".to_string(),
            NotificationChannel::Email,
        );
        assert_eq!(notification.recipient, "user@example.com");
        assert_eq!(notification.subject, "Test Subject");
    }

    #[test]
    fn test_email_notifier() {
        let notifier = EmailNotifier;
        let notification = Notification::new(
            "user@example.com".to_string(),
            "Test".to_string(),
            "Body".to_string(),
            NotificationChannel::Email,
        );
        assert!(notifier.send(&notification));
    }

    #[test]
    fn test_slack_notifier() {
        let notifier = SlackNotifier;
        let notification = Notification::new(
            "@user".to_string(),
            "Test".to_string(),
            "Body".to_string(),
            NotificationChannel::Slack,
        );
        assert!(notifier.send(&notification));
    }

    #[test]
    fn test_cli_notifier() {
        let notifier = CliNotifier;
        let notification = Notification::new(
            "user".to_string(),
            "Test".to_string(),
            "Body".to_string(),
            NotificationChannel::Cli,
        );
        assert!(notifier.send(&notification));
    }

    #[test]
    fn test_notification_service() {
        let mut service = NotificationService::new();
        let notification = Notification::new(
            "user@example.com".to_string(),
            "Test".to_string(),
            "Body".to_string(),
            NotificationChannel::Email,
        );
        assert!(service.send_notification(notification));
    }

    // Submission Notification Tests
    #[test]
    fn test_notify_approvers() {
        let mut service = NotificationService::new();
        let approvers = vec!["approver1@example.com".to_string(), "approver2@example.com".to_string()];
        let sent = notify_approvers(
            &mut service,
            "change-1",
            "Test Change",
            "Testing submission notifications",
            "Medium",
            approvers.clone(),
        );
        assert_eq!(sent.len(), 2);
    }

    // Decision Notification Tests
    #[test]
    fn test_notify_submitter_approved() {
        let mut service = NotificationService::new();
        let result = notify_submitter(
            &mut service,
            "submitter@example.com".to_string(),
            "change-1",
            "Test Change",
            "approved",
            "approver@example.com",
            None,
        );
        assert!(result);
    }

    #[test]
    fn test_notify_submitter_rejected() {
        let mut service = NotificationService::new();
        let result = notify_submitter(
            &mut service,
            "submitter@example.com".to_string(),
            "change-1",
            "Test Change",
            "rejected",
            "approver@example.com",
            Some("Needs more testing"),
        );
        assert!(result);
    }

    // Preference Tests
    #[test]
    fn test_default_preferences() {
        let prefs = NotificationPreferences::new();
        assert!(prefs.is_enabled(&NotificationChannel::Email));
        assert!(prefs.is_enabled(&NotificationChannel::Slack));
        assert!(prefs.is_enabled(&NotificationChannel::Cli));
    }

    #[test]
    fn test_preference_service() {
        let mut service = PreferenceService::new();
        let prefs = service.get_preferences("user1");
        assert!(prefs.is_enabled(&NotificationChannel::Email));
    }

    // End-to-End Workflow Test
    #[test]
    fn test_complete_notification_workflow() {
        let mut service = NotificationService::new();
        let mut pref_service = PreferenceService::new();

        // Submit change and notify approvers
        let approvers = vec!["approver@example.com".to_string()];
        let sent = notify_approvers(
            &mut service,
            "change-1",
            "Test Change",
            "Testing complete workflow",
            "Medium",
            approvers,
        );
        assert_eq!(sent.len(), 1);

        // Approve and notify submitter
        let result = notify_submitter(
            &mut service,
            "submitter@example.com".to_string(),
            "change-1",
            "Test Change",
            "approved",
            "approver@example.com",
            None,
        );
        assert!(result);

        // Verify preferences were accessible
        let prefs = pref_service.get_preferences("user1");
        assert!(prefs.is_enabled(&NotificationChannel::Email));
    }
}
