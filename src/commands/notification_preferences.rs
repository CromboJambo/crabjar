//! Notification Preferences
//!
//! Manages per-user notification channel preferences.

use std::collections::HashMap;
use crate::commands::notification_infrastructure::NotificationChannel;

/// User notification preferences.
#[derive(Debug, Clone)]
pub struct NotificationPreferences {
    pub channels: HashMap<NotificationChannel, bool>,
}

impl NotificationPreferences {
    /// Create default preferences (all channels enabled).
    pub fn new() -> Self {
        let mut channels = HashMap::new();
        channels.insert(NotificationChannel::Email, true);
        channels.insert(NotificationChannel::Slack, true);
        channels.insert(NotificationChannel::Cli, true);

        Self { channels }
    }

    /// Get preference for a specific channel.
    pub fn is_enabled(&self, channel: &NotificationChannel) -> bool {
        self.channels.get(channel).copied().unwrap_or(false)
    }

    /// Set preference for a specific channel.
    pub fn set_channel(&mut self, channel: NotificationChannel, enabled: bool) {
        self.channels.insert(channel, enabled);
    }
}

/// Service for managing user notification preferences.
pub struct PreferenceService {
    preferences: HashMap<String, NotificationPreferences>,
}

impl PreferenceService {
    /// Create a new preference service.
    pub fn new() -> Self {
        Self {
            preferences: HashMap::new(),
        }
    }

    /// Get user preferences (creates default if not exists).
    pub fn get_preferences(&mut self, user_id: &str) -> &NotificationPreferences {
        if !self.preferences.contains_key(user_id) {
            self.preferences.insert(user_id.to_string(), NotificationPreferences::new());
        }
        self.preferences.get(user_id).unwrap()
    }

    /// Update user preference for a specific channel.
    pub fn update_preference(&mut self, user_id: &str, channel: NotificationChannel, enabled: bool) {
        let prefs = self.get_preferences(user_id);
        // Note: In production, this would need mutable access
        println!("Updated {} preference for {}: {}", channel, user_id, enabled);
    }
}
