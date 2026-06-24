/// `mqtt` — MQTT integration module for `CrabJar`.
///
/// Provides MQTT client, discovery, and media bridge functionality.
/// and command reception. Mirrors the Teams-for-Linux MQTT integration:
/// - Status publishing: presence, call state, camera, mic, screen-sharing
/// - Command reception: toggle-mute, toggle-video, toggle-hand-raise
/// - Home Assistant auto-discovery
/// - Last Will and Testament (LWT) for connection state
/// - Automatic reconnection with backoff
pub mod client;
pub mod config;
pub mod discovery;
pub mod handler;
pub mod media_bridge;

pub use client::MqttClient;
pub use client::MqttError;
pub use config::{MqttConfig, HomeAssistantConfig};
pub use discovery::HaDiscovery;
pub use handler::CommandHandler;
pub use media_bridge::{MediaBridge, MediaEvent};

/// Status codes matching Teams-for-Linux convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Unknown = -1,
    Available = 1,
    Busy = 2,
    DoNotDisturb = 3,
    Away = 4,
    BeRightBack = 5,
}

impl PresenceStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            PresenceStatus::Unknown => "unknown",
            PresenceStatus::Available => "available",
            PresenceStatus::Busy => "busy",
            PresenceStatus::DoNotDisturb => "do_not_disturb",
            PresenceStatus::Away => "away",
            PresenceStatus::BeRightBack => "be_right_back",
        }
    }

    #[must_use]
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => PresenceStatus::Available,
            2 => PresenceStatus::Busy,
            3 => PresenceStatus::DoNotDisturb,
            4 => PresenceStatus::Away,
            5 => PresenceStatus::BeRightBack,
            _ => PresenceStatus::Unknown,
        }
    }
}

/// Presence status update published to MQTT.
///
/// Includes deduplication support: `should_publish()` checks whether this
/// update differs from the last published status, mirroring the Electron
/// app's `lastPublishedStatus` check in mqtt/index.js.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresenceUpdate {
    pub status: String,
    pub status_code: i32,
    pub timestamp: String,
    pub client_id: String,
}

impl PresenceUpdate {
    /// Check whether this update should be published.
    ///
    /// Returns `false` if `last_published_status` matches the current status,
    /// indicating a duplicate that should be skipped. Returns `true` if the
    /// status has changed or no previous status exists.
    #[must_use]
    pub fn should_publish(&self, last_published_status: Option<&str>) -> bool {
        match last_published_status {
            Some(prev) if prev == self.status => false,
            _ => true,
        }
    }
}

/// Media state for MQTT publishing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MediaState {
    Camera { enabled: bool },
    Microphone { state: String },
    InCall { in_call: bool },
    ScreenSharing { sharing: bool },
}

/// MQTT event types for the event bus bridge.
#[derive(Debug, Clone)]
pub enum MqttEvent {
    Connected,
    Disconnected,
    StatusChanged(PresenceUpdate),
    MediaStateChanged(MediaState),
    CommandReceived { action: String, request_id: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_update(status: &str) -> PresenceUpdate {
        PresenceUpdate {
            status: status.to_string(),
            status_code: 1,
            timestamp: "2026-01-01T00:00:00Z".into(),
            client_id: "test".into(),
        }
    }

    #[test]
    fn test_dedup_no_previous() {
        let update = make_update("available");
        assert!(update.should_publish(None));
    }

    #[test]
    fn test_dedup_different_status() {
        let update = make_update("busy");
        assert!(update.should_publish(Some("available")));
    }

    #[test]
    fn test_dedup_same_status() {
        let update = make_update("busy");
        assert!(!update.should_publish(Some("busy")));
    }
}
