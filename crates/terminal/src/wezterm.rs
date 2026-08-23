//! Wezterm mux backend implementation.
//!
//! Terminal backend for wezterm via its built-in mux protocol.
//!
//! Provides a unified API for spawning, controlling, and recording terminal sessions
//! via wezterm's `wezterm cli` command which communicates with the background mux server.

use std::path::Path;

use anyhow::Context;
use tokio::process::Command;

use crate::backend::{SpawnResult, TerminalBackend};

/// Wezterm mux backend for terminal session management.
#[derive(Debug, Clone)]
pub struct WeztermBackend {
    /// Class name for targeting specific wezterm instances (optional)
    class: Option<String>,
}

impl WeztermBackend {
    /// Create a new WeztermBackend with default settings.
    pub fn new() -> Self {
        Self { class: None }
    }

    /// Set the class name for targeting specific wezterm instances.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Build the base `wezterm cli` command with optional class targeting.
    fn build_cli_cmd(&self) -> Command {
        let mut cmd = Command::new("wezterm");
        cmd.arg("cli")
            .arg("--no-auto-start") // Don't auto-start GUI, just connect to mux
            .arg("--prefer-mux"); // Prefer connecting to background mux server

        if let Some(ref class) = self.class {
            cmd.arg("--class").arg(class);
        }

        cmd
    }

    /// Execute a wezterm cli command and return stdout.
    async fn run_cli(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = self.build_cli_cmd();
        cmd.args(args);

        let output = cmd
            .output()
            .await
            .with_context(|| "Failed to execute wezterm cli")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("wezterm cli failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Parse a pane ID from wezterm spawn output.
    /// Wezterm outputs the pane-id on success, typically in format like "pane_12345".
    fn parse_pane_id(output: &str) -> Option<String> {
        // Try to extract pane ID from output (could be JSON or plain text)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            json.get("pane_id")
                .or_else(|| json.get("PaneId"))
                .and_then(|v| v.as_str())
                .map(String::from)
        } else {
            // Fallback: try to find pane ID pattern in output
            let output_lower = output.to_lowercase();
            if output_lower.contains("pane") || output_lower.contains("window") {
                Some(output.trim().to_string())
            } else {
                None
            }
        }
    }
}

impl Default for WeztermBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TerminalBackend for WeztermBackend {
    fn name(&self) -> &str {
        "wezterm"
    }

    fn is_available() -> bool {
        which::which("wezterm").is_ok()
    }

    async fn spawn(&self, session_name: &str, working_dir: &Path) -> anyhow::Result<SpawnResult> {
        let mut cmd = self.build_cli_cmd();

        // Spawn a new window with the given command in detached mode
        cmd.arg("spawn")
            .arg("--cwd")
            .arg(working_dir)
            .arg("--")
            .arg("bash");

        tracing::info!(session = %session_name, working_dir = ?working_dir, "spawning wezterm session");

        let output = cmd
            .output()
            .await
            .with_context(|| "Failed to spawn wezterm window")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("wezterm spawn failed: {}", stderr);
        }

        // Parse pane ID from output
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let pane_id = Self::parse_pane_id(&stdout);

        Ok(SpawnResult {
            pane_id,
            session_name: session_name.to_string(),
        })
    }

    async fn send_text(&self, _session_name: &str, input: &str) -> anyhow::Result<()> {
        // We need to target a specific pane. For now, we'll use the first available pane.
        // In a full implementation, you'd track pane IDs per session.

        let list_output = self.run_cli(&["list", "--output-format", "json"]).await?;

        // Parse the list output to find panes in this session
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&list_output)
            && let Some(panes) = json
                .get("windows")
                .and_then(|w| w.get(0))
                .and_then(|w| w.get("panes"))
            && let Some(pane_array) = panes.as_array()
            && let Some(first_pane) = pane_array.first()
            && let Some(pane_id) = first_pane.get("id").and_then(|p| p.as_u64())
        {
            // Send text to the specific pane
            self.run_cli(&["send-text", "--pane-id", &pane_id.to_string(), input])
                .await?;
            return Ok(());
        }

        // Fallback: send to focused pane without specific targeting
        let _ = self.run_cli(&["send-text", input]).await?;
        Ok(())
    }

    async fn read_output(&self, _session_name: &str, _lines: usize) -> anyhow::Result<String> {
        // Get text from the first available pane in the session
        let list_output = self.run_cli(&["list", "--output-format", "json"]).await?;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&list_output)
            && let Some(panes) = json
                .get("windows")
                .and_then(|w| w.get(0))
                .and_then(|w| w.get("panes"))
            && let Some(pane_array) = panes.as_array()
            && let Some(first_pane) = pane_array.first()
            && let Some(pane_id) = first_pane.get("id").and_then(|p| p.as_u64())
        {
            // Get text from the specific pane
            return self
                .run_cli(&["get-text", "--pane-id", &pane_id.to_string()])
                .await;
        }

        // Fallback: get text from focused pane
        self.run_cli(&["get-text"]).await
    }

    async fn kill_session(&self, session_name: &str) -> anyhow::Result<()> {
        // Kill all windows in the workspace/session
        let output = self.run_cli(&["list", "--output-format", "json"]).await?;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output)
            && let Some(windows) = json.get("windows").and_then(|w| w.as_array())
        {
            for _window in windows {
                // Kill each window
                let _ = self.run_cli(&["cli", "kill-window"]).await;
            }
        }

        tracing::info!(session = %session_name, "killed wezterm session");
        Ok(())
    }

    async fn split_pane_horizontal(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        let mut cmd = self.build_cli_cmd();

        // Split pane horizontally (top/bottom layout)
        cmd.arg("split-pane").arg("--direction").arg("down");

        if let Some(dir) = working_dir {
            cmd.arg("--cwd").arg(dir);
        }

        tracing::info!(session = %session_name, "splitting wezterm pane horizontally");

        let output = cmd
            .output()
            .await
            .with_context(|| "Failed to split wezterm pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("wezterm split-pane failed: {}", stderr);
        }

        // Parse new pane ID from output (if available)
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let pane_id = Self::parse_pane_id(&stdout);

        Ok(pane_id.unwrap_or_else(|| "split".to_string()))
    }

    async fn split_pane_vertical(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        let mut cmd = self.build_cli_cmd();

        // Split pane vertically (left/right layout)
        cmd.arg("split-pane").arg("--direction").arg("right");

        if let Some(dir) = working_dir {
            cmd.arg("--cwd").arg(dir);
        }

        tracing::info!(session = %session_name, "splitting wezterm pane vertically");

        let output = cmd
            .output()
            .await
            .with_context(|| "Failed to split wezterm pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("wezterm split-pane failed: {}", stderr);
        }

        // Parse new pane ID from output (if available)
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let pane_id = Self::parse_pane_id(&stdout);

        Ok(pane_id.unwrap_or_else(|| "split".to_string()))
    }
}
