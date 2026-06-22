/// Command handler for MQTT-received commands from Home Assistant or other sources.
///
/// Handles Teams-for-Linux compatible commands:
/// - `toggle-mute` — toggle microphone mute state
/// - `toggle-video` — toggle camera on/off
/// - `toggle-hand-raise` — toggle hand raise in meetings
/// - `start-screen-share` / `stop-screen-share` — screen sharing control
/// - `get-calendar` — fetch calendar events (non-shortcut action)
///
/// Security: validates incoming commands against an allowed-actions whitelist
/// (mirrors the Electron app's `allowedActions` check in mqtt/index.js).

use tracing::{debug, warn};

/// Allowed MQTT command actions (mirrors Electron's `allowedActions`).
pub const ALLOWED_ACTIONS: &[&str] = &[
    "toggle-mute",
    "toggle-video",
    "toggle-hand-raise",
    "start-screen-share",
    "stop-screen-share",
    "get-calendar",
];

/// Handler for incoming MQTT commands.
pub struct CommandHandler {
    /// Callback for toggle-mute
    on_toggle_mute: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
    /// Callback for toggle-video
    on_toggle_video: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
    /// Callback for toggle-hand-raise
    on_toggle_hand_raise: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
    /// Callback for start screen share
    on_start_screen_share: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
    /// Callback for stop screen share
    on_stop_screen_share: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
    /// Callback for get-calendar
    on_get_calendar: Option<tokio::sync::Mutex<Option<Box<dyn Fn() -> tokio::task::JoinHandle<()> + Send>>>>,
}

impl CommandHandler {
    /// Create a new command handler with no callbacks registered.
    pub fn new() -> Self {
        Self {
            on_toggle_mute: None,
            on_toggle_video: None,
            on_toggle_hand_raise: None,
            on_start_screen_share: None,
            on_stop_screen_share: None,
            on_get_calendar: None,
        }
    }

    /// Register a callback for toggle-mute commands.
    pub fn on_toggle_mute<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_toggle_mute = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Register a callback for toggle-video commands.
    pub fn on_toggle_video<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_toggle_video = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Register a callback for toggle-hand-raise commands.
    pub fn on_toggle_hand_raise<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_toggle_hand_raise = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Register a callback for start-screen-share commands.
    pub fn on_start_screen_share<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_start_screen_share = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Register a callback for stop-screen-share commands.
    pub fn on_stop_screen_share<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_stop_screen_share = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Register a callback for get-calendar commands.
    pub fn on_get_calendar<F>(&mut self, callback: F)
    where
        F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.on_get_calendar = Some(tokio::sync::Mutex::new(Some(Box::new(callback))));
    }

    /// Handle an incoming MQTT command.
    ///
    /// Validates the action against the allowed-actions whitelist before dispatching.
    /// Returns `Some(action)` if the command was handled, `None` if it was rejected
    /// or unknown.
    pub async fn handle(&self, action: &str, request_id: Option<String>) -> bool {
        // Whitelist validation (mirrors Electron's `allowedActions.includes()` check)
        if !ALLOWED_ACTIONS.contains(&action) {
            warn!(action, "Rejected MQTT command: action not in whitelist");
            return false;
        }

        debug!(action, ?request_id, "Handling MQTT command");

        match action {
            "toggle-mute" => {
                if let Some(tx) = &self.on_toggle_mute {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            "toggle-video" => {
                if let Some(tx) = &self.on_toggle_video {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            "toggle-hand-raise" => {
                if let Some(tx) = &self.on_toggle_hand_raise {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            "start-screen-share" => {
                if let Some(tx) = &self.on_start_screen_share {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            "stop-screen-share" => {
                if let Some(tx) = &self.on_stop_screen_share {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            "get-calendar" => {
                if let Some(tx) = &self.on_get_calendar {
                    let guard = tx.lock().await;
                    if let Some(cb) = guard.as_ref() {
                        let _ = cb();
                    }
                }
            }
            _ => {
                // Should not reach here due to whitelist check above, but handle defensively.
                warn!(action, "Unknown MQTT command received");
                return false;
            }
        }

        true
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_handler_creation() {
        let handler = CommandHandler::new();
        assert!(handler.on_toggle_mute.is_none());
        assert!(handler.on_toggle_video.is_none());
        assert!(handler.on_get_calendar.is_none());
    }

    #[test]
    fn test_default_command_handler() {
        let handler = CommandHandler::default();
        assert!(handler.on_toggle_mute.is_none());
    }

    #[test]
    fn test_whitelist_contains_expected_actions() {
        assert!(ALLOWED_ACTIONS.contains(&"toggle-mute"));
        assert!(ALLOWED_ACTIONS.contains(&"toggle-video"));
        assert!(ALLOWED_ACTIONS.contains(&"toggle-hand-raise"));
        assert!(ALLOWED_ACTIONS.contains(&"start-screen-share"));
        assert!(ALLOWED_ACTIONS.contains(&"stop-screen-share"));
        assert!(ALLOWED_ACTIONS.contains(&"get-calendar"));
    }

    #[test]
    fn test_whitelist_rejects_unknown_action() {
        assert!(!ALLOWED_ACTIONS.contains(&"unknown-action"));
        assert!(!ALLOWED_ACTIONS.contains(&"delete-meeting"));
        assert!(!ALLOWED_ACTIONS.contains(&""));
    }
}
