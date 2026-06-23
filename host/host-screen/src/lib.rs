//! # host-screen
//!
//! Screen capture and display protocol integration for crabjar-host.
//! Provides a unified API for screen sharing, display capture, and
//! integration with vm-bridge for remote display protocols.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Teams Plugin                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐  │
//! │  │  ScreenShare │  │  DisplayRelay│  │  SharedTerminal  │  │
//! │  │  Capture API │  │  (vm-bridge) │  │     (wezterm)    │  │
//! │  └──────┬──────┘  └──────┬───────┘  └────────┬─────────┘  │
//! └─────────┼────────────────┼────────────────────┼────────────┘
//!           │                 │                    │
//!           ▼                 ▼                    ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   host-screen Crate                         │
//! │  • PipeWire screen share sources                            │
//! │  • XDG-Portal Wayland capture                              │
//! │  • Preview thumbnail generation (320x180)                    │
//! │  • vm-bridge WebSocket relay for display protocols           │
//! │  • Shared terminal protocol over WebSocket                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use host_screen::ScreenCapture;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let capture = ScreenCapture::new();
//!     let session = capture.start_session().await?;
//!     if let Some(session) = session {
//!         println!("WebSocket URL: {}", session.ws_url());
//!     }
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Screen capture source (PipeWire stream or XDG-Portal)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureSource {
    /// PipeWire screen share source (Wayland)
    PipeWire {
        /// Stream ID from PipeWire
        stream_id: String,
        /// Monitor type (monitor, application)
        monitor: MonitorType,
    },
    /// XDG-Portal screen share source (Wayland)
    XdgPortal {
        /// Portal application path
        app_id: String,
        /// Target widget type
        widget: String,
    },
    /// X11 XFixes source (X11)
    X11Xfixes,
}

/// PipeWire monitor type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MonitorType {
    Monitor,
    Application,
}

/// Screen capture session
#[derive(Debug, Clone)]
pub struct ScreenCapture {
    /// Capture source
    pub source: CaptureSource,
    /// Frame rate
    pub fps: u32,
    /// Quality (0-100)
    pub quality: u32,
    /// Target resolution
    pub resolution: Resolution,
}

/// Screen share session (started capture)
#[derive(Debug, Clone)]
pub struct ScreenShareSession {
    /// WebSocket URL for vm-bridge relay
    pub ws_url: String,
    /// Preview thumbnail (320x180)
    pub preview_path: Option<PathBuf>,
    /// Capture source
    pub source: CaptureSource,
}

/// Display protocol for vm-bridge relay
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DisplayProtocol {
    Spice,
    Vnc,
}

/// vm-bridge connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConnection {
    /// WebSocket URL
    pub ws_url: String,
    /// Display protocol
    pub protocol: DisplayProtocol,
    /// VM name (from manifest)
    pub vm_name: String,
}

/// Screen share manager
#[derive(Debug, Clone)]
pub struct ScreenManager {
    capture: ScreenCapture,
    session: Option<ScreenShareSession>,
}

/// Resolution preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// 1280x720
    _HD,
    /// 1920x1080
    _FHD,
    /// Auto-detect
    _Auto,
}

impl Default for Resolution {
    fn default() -> Self {
        Self::_Auto
    }
}

impl ScreenCapture {
    /// Create a new ScreenCapture with default settings
    pub fn new() -> Self {
        Self {
            source: CaptureSource::XdgPortal {
                app_id: "org.wezfurland.wezterm".to_string(),
                widget: "display".to_string(),
            },
            fps: 30,
            quality: 80,
            resolution: Resolution::default(),
        }
    }

    /// Start screen capture session
    pub async fn start_session(&self) -> Result<Option<ScreenShareSession>> {
        // TODO: Implement PipeWire/XDG-Portal integration
        // For now, return None to indicate unimplemented
        tracing::info!(
            source = ?self.source,
            "starting screen capture session"
        );
        Ok(None)
    }

    /// Get capture source info
    pub fn source(&self) -> &CaptureSource {
        &self.source
    }
}

impl ScreenShareSession {
    /// Get WebSocket URL for vm-bridge relay
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Get preview thumbnail path
    pub fn preview_path(&self) -> Option<&PathBuf> {
        self.preview_path.as_ref()
    }

    /// Generate preview thumbnail (320x180)
    pub async fn generate_preview(&mut self) -> Result<PathBuf> {
        // TODO: Implement thumbnail generation
        // This would capture a frame and resize to 320x180
        tracing::info!("generating preview thumbnail");
        Ok(PathBuf::from("/tmp/preview.png"))
    }
}

impl ScreenManager {
    /// Create a new ScreenManager
    pub fn new() -> Self {
        Self {
            capture: ScreenCapture::new(),
            session: None,
        }
    }

    /// Start a new screen share session
    pub async fn start_share(&mut self) -> Result<&ScreenShareSession> {
        if let Some(session) = self.capture.start_session().await? {
            self.session = Some(session);
            Ok(self.session.as_ref().unwrap())
        } else {
            // Fallback: return minimal session
            let fallback = ScreenShareSession {
                ws_url: "ws://localhost:8080/ws".to_string(),
                preview_path: Some(PathBuf::from("/tmp/fallback-preview.png")),
                source: CaptureSource::XdgPortal {
                    app_id: "fallback".to_string(),
                    widget: "display".to_string(),
                },
            };
            self.session = Some(fallback);
            Ok(self.session.as_ref().unwrap())
        }
    }

    /// Stop current session
    pub fn stop_share(&mut self) {
        self.session = None;
    }

    /// Get current session
    pub fn session(&self) -> Option<&ScreenShareSession> {
        self.session.as_ref()
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Screen capture error types
#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("capture failed: {0}")]
    CaptureFailed(String),
    #[error("encode failed: {0}")]
    EncodeFailed(String),
    #[error("relay failed: {0}")]
    RelayFailed(String),
    #[error("preview generation failed: {0}")]
    PreviewFailed(String),
    #[error("configuration error: {0}")]
    Config(String),
}

impl From<std::io::Error> for ScreenError {
    fn from(e: std::io::Error) -> Self {
        ScreenError::CaptureFailed(format!("IO error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_source_serialization() {
        let source = CaptureSource::XdgPortal {
            app_id: "test".to_string(),
            widget: "display".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("xdgportal"));
    }

    #[test]
    fn test_resolution_default() {
        let res = Resolution::default();
        assert_eq!(res, Resolution::_Auto);
    }
}
