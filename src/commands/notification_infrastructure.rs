//! Notification Infrastructure
//!
//! Provides the core notification system with channel abstraction and retry logic.

use std::collections::HashMap;
use chrono::Utc;

/// Represents a notification message.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: String,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub channel: NotificationChannel,
    pub sent_at: Option<i64>,
    pub status: NotificationStatus,
}

/// Supported notification channels.
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationChannel {
    Email,
    Slack,
    Cli,
}

/// Notification delivery status.
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Failed,
}

impl Notification {
    /// Create a new notification.
    pub fn new(recipient: String, subject: String, body: String, channel: NotificationChannel) -> Self {
        Self {
            id: format!("notif-{}", Utc::now().timestamp()),
            recipient,
            subject,
            body,
            channel,
            sent_at: None,
            status: NotificationStatus::Pending,
        }
    }

    /// Mark notification as sent.
    pub fn mark_sent(&mut self) {
        self.sent_at = Some(Utc::now().timestamp());
        self.status = NotificationStatus::Sent;
    }

    /// Mark notification as failed.
    pub fn mark_failed(&mut self) {
        self.status = NotificationStatus::Failed;
    }
}

/// Notifier interface for sending notifications.
pub trait Notifier {
    /// Send a notification to the specified recipient.
    fn send(&self, notification: &Notification) -> bool;
}

/// Email notifier implementation (simplified).
pub struct EmailNotifier;

impl Notifier for EmailNotifier {
    fn send(&self, notification: &Notification) -> bool {
        // In production, this would use an SMTP library or API
        println!("[EMAIL] To: {}, Subject: {}", notification.recipient, notification.subject);
        true
    }
}

/// Slack notifier implementation (simplified).
pub struct SlackNotifier;

impl Notifier for SlackNotifier {
    fn send(&self, notification: &Notification) -> bool {
        // In production, this would use the Slack API
        println!("[SLACK] To: {}, Message: {}", notification.recipient, notification.subject);
        true
    }
}

/// CLI notifier implementation (simplified).
pub struct CliNotifier;

impl Notifier for CliNotifier {
    fn send(&self, notification: &Notification) -> bool {
        println!("[CLI] To: {}, Message: {}", notification.recipient, notification.subject);
        true
    }
}

/// Notification service with retry logic.
pub struct NotificationService {
    notifiers: HashMap<NotificationChannel, Box<dyn Notifier>>,
    notifications: Vec<Notification>,
}

impl NotificationService {
    /// Create a new notification service.
    pub fn new() -> Self {
        let mut notifiers = HashMap::new();
        notifiers.insert(NotificationChannel::Email, Box::new(EmailNotifier));
        notifiers.insert(NotificationChannel::Slack, Box::new(SlackNotifier));
        notifiers.insert(NotificationChannel::Cli, Box::new(CliNotifier));

        Self {
            notifiers,
            notifications: Vec::new(),
        }
    }

    /// Send a notification with retry logic.
    pub fn send_notification(&mut self, mut notification: Notification) -> bool {
        let max_retries = 3;
        let mut attempts = 0;

        loop {
            if let Some(notifier) = self.notifiers.get(&notification.channel) {
                if notifier.send(&notification) {
                    notification.mark_sent();
                    break true;
                }
            }

            attempts += 1;
            if attempts >= max_retries {
                notification.mark_failed();
                break false;
            }
        }
    }

    /// Record a notification for later processing.
    pub fn record_notification(&mut self, notification: Notification) -> String {
        self.notifications.push(notification);
        self.notifications.last().unwrap().id.clone()
    }
}
