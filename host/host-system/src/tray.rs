/// System tray management.
///
/// Provides a minimal tray API that can be backed by Tauri (Linux/macOS/Windows)
/// or a native library (libappindicator on Linux).

use crabjar_host_core::event_bus::EventBus;
use std::sync::Arc;

/// System tray handle.
///
/// On Linux this uses libappindicator or libayatana-appindicator.
/// On macOS/Windows it delegates to Tauri's tray plugin.
pub struct SystemTray {
    event_bus: Arc<EventBus>,
    icon_path: Option<String>,
    visible: bool,
    badge_count: u32,
}

impl SystemTray {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            icon_path: None,
            visible: false,
            badge_count: 0,
        }
    }

    /// Set the tray icon path.
    pub fn set_icon(&mut self, path: String) {
        self.icon_path = Some(path);
    }

    /// Set the unread / badge count displayed on the tray icon.
    pub fn set_badge(&mut self, count: u32) {
        self.badge_count = count;
    }

    /// Show the tray icon.
    pub async fn show(&mut self) -> Result<(), TrayError> {
        self.visible = true;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::TrayChanged {
                action: "shown".into(),
            },
            "tray",
        );
        tracing::info!("system tray shown");
        Ok(())
    }

    /// Hide the tray icon.
    pub async fn hide(&mut self) -> Result<(), TrayError> {
        self.visible = false;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::TrayChanged {
                action: "hidden".into(),
            },
            "tray",
        );
        tracing::info!("system tray hidden");
        Ok(())
    }

    /// Check if the tray is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the current icon path.
    pub fn icon_path(&self) -> Option<&str> {
        self.icon_path.as_deref()
    }

    /// Get the current badge count.
    pub fn badge_count(&self) -> u32 {
        self.badge_count
    }

    /// Add a menu item handler.
    ///
    /// The action string is emitted as a UserInput event when clicked.
    pub fn on_menu_action(&self, action: &str) {
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::UserInput {
                input: format!("tray:{}", action),
            },
            "tray",
        );
    }
}

/// Tray errors.
#[derive(thiserror::Error, Debug)]
pub enum TrayError {
    #[error("icon not found: {0}")]
    IconNotFound(String),
    #[error("failed to create tray: {0}")]
    CreationFailed(String),
    #[error("tray already exists")]
    AlreadyExists,
    #[error("tray not initialized")]
    NotInitialized,
}
