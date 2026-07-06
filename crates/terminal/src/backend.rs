//! Terminal backend trait and detection utilities.
//!
//! Defines the `TerminalBackend` trait that all multiplexer backends must implement,
//! plus utility functions for detecting available terminal tools on the system.

use std::path::Path;

/// Spawn result containing pane/session metadata
#[derive(Debug, Clone)]
pub struct SpawnResult {
    /// Pane ID (backend-specific format)
    pub pane_id: Option<String>,
    /// Session name that was created
    pub session_name: String,
}

/// Backend-agnostic terminal operations trait.
///
/// All multiplexer backends (wezterm, zellij) implement this trait to provide
/// a unified API for spawning sessions, sending input, reading output, and
/// managing lifecycle.
#[async_trait::async_trait]
pub trait TerminalBackend: std::fmt::Debug + Send + Sync {
    /// Return the backend name (e.g., "wezterm", "zellij")
    fn name(&self) -> &str;

    /// Check if this backend is available on the system
    fn is_available() -> bool where Self: Sized;

    /// Spawn a new detached terminal session.
    ///
    /// Returns a `SpawnResult` with pane/session metadata.
    async fn spawn(
        &self,
        session_name: &str,
        working_dir: &Path,
    ) -> anyhow::Result<SpawnResult>;

    /// Send text/keys to an active terminal session's focused pane.
    async fn send_text(&self, session_name: &str, input: &str) -> anyhow::Result<()>;

    /// Read the last N lines of terminal output from a session.
    async fn read_output(
        &self,
        session_name: &str,
        lines: usize,
    ) -> anyhow::Result<String>;

    /// Kill/terminate an entire terminal session.
    async fn kill_session(&self, session_name: &str) -> anyhow::Result<()>;

    /// Split the current pane horizontally (top/bottom layout).
    async fn split_pane_horizontal(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String>;

    /// Split the current pane vertically (left/right layout).
    async fn split_pane_vertical(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String>;
}

/// Detect which terminal multiplexers are available on the system.
///
/// Returns a list of available backend names in priority order
/// (wezterm first, then zellij).
pub fn detect_available_backends() -> Vec<&'static str> {
    let mut available = Vec::new();

    // Check for wezterm and zellij directly here to avoid circular imports
    if which::which("wezterm").is_ok() {
        available.push("wezterm");
    }

    if which::which("zellij").is_ok() {
        available.push("zellij");
    }

    available
}

/// Auto-detect the best available backend.
///
/// Returns `Ok(backend_name)` if a multiplexer is found, or an error
/// explaining which tools are missing.
pub fn auto_detect_backend() -> anyhow::Result<&'static str> {
    let available = detect_available_backends();

    match available.first() {
        Some(&name) => Ok(name),
        None => anyhow::bail!(
            "No terminal multiplexer found. Install wezterm or zellij.\n\
             See: https://wezfurlong.org/wezterm/install.html\n\
             Or:  https://zellij.dev/documentation/getting-started/"
        ),
    }
}
