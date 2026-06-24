/// Desktop notification service.
///
/// Uses libnotify on Linux, Tauri notification plugin on macOS/Windows.
use crabjar_host_core::event_bus::EventBus;
use std::sync::Arc;

/// Notification categories supported by Teams.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum NotificationCategory {
    /// New message notification
    #[default]
    Message,
    /// Incoming call notification
    IncomingCall,
    /// System / status change notification
    System,
}


/// Metadata attached to a notification.
#[derive(Debug, Clone)]
pub struct NotificationMeta {
    pub category: NotificationCategory,
    pub app_id: String,
    pub icon: Option<String>,
}

impl Default for NotificationMeta {
    fn default() -> Self {
        Self {
            category: NotificationCategory::default(),
            app_id: "crabjar-host".into(),
            icon: None,
        }
    }
}

pub struct NotificationService {
    event_bus: Arc<EventBus>,
    enabled: bool,
    default_timeout: i32,
}

impl NotificationService {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            enabled: true,
            default_timeout: 5000, // milliseconds
        }
    }

    /// Send a desktop notification.
    ///
    /// On Linux this uses libnotify. On other platforms it emits a
    /// `Notification` event for the host to handle.
    pub fn notify(
        &self,
        title: &str,
        body: &str,
        timeout: Option<i32>,
    ) -> Result<(), NotificationError> {
        let timeout = timeout.unwrap_or(self.default_timeout);

        // On Linux, use libnotify
        #[cfg(target_os = "linux")]
        {
            if self.enabled {
                let notification = libnotify::Notification::new(
                    "crabjar-host",
                    title,
                    body,
                );
                notification.set_timeout(timeout);
                if let Err(e) = notification.show() {
                    tracing::warn!(error = %e, "failed to show notification");
                    return Err(NotificationError::SendFailed(e.to_string()));
                }
            }
        }

        // Emit event for internal tracking
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::Notification {
                title: title.to_string(),
                body: body.to_string(),
            },
            "notification-service",
        );

        tracing::info!(title, body, "notification sent");
        Ok(())
    }

    /// Send an incoming call notification.
    pub fn notify_incoming_call(
        &self,
        from: &str,
        title: &str,
    ) -> Result<(), NotificationError> {
        self.notify(
            title,
            &format!("Call from {}", from),
            Some(-1), // persistent until dismissed
        )
    }

    /// Send a system status notification.
    pub fn notify_system(
        &self,
        title: &str,
        body: &str,
    ) -> Result<(), NotificationError> {
        self.notify(title, body, Some(3000))
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Notification errors.
#[derive(thiserror::Error, Debug)]
pub enum NotificationError {
    #[error("notification service not available")]
    NotAvailable,
    #[error("failed to send notification: {0}")]
    SendFailed(String),
}
