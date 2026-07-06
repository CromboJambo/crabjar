//! Zellij action protocol backend implementation.
//!
//! Terminal backend for zellij via its CLI action protocol.
//!
//! Provides a unified API for spawning, controlling, and recording terminal sessions
//! via zellij's `zellij action` command which communicates with the running server.

use std::path::Path;

use anyhow::Context;
use tokio::process::Command;

use crate::backend::{SpawnResult, TerminalBackend};

/// Zellij action protocol backend for terminal session management.
#[derive(Debug, Clone)]
pub struct ZellijBackend {
    /// Session name prefix for grouping related panes (optional)
    session_prefix: Option<String>,
}

impl ZellijBackend {
    /// Create a new ZellijBackend with default settings.
    pub fn new() -> Self {
        Self {
            session_prefix: None,
        }
    }

    /// Set the session name prefix for grouping related panes (optional).
    pub fn with_session_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.session_prefix = Some(prefix.into());
        self
    }

    /// Build a zellij command with optional session targeting.
    fn build_cmd(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("zellij");
        
        if let Some(ref prefix) = self.session_prefix {
            // Use the prefixed session name for all operations
            cmd.arg("-s").arg(prefix);
        }
        
        cmd.args(args);
        cmd
    }

    /// Execute a zellij command and return stdout.
    #[allow(dead_code)]
    async fn run_zellij(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = self.build_cmd(args);

        tracing::debug!(?args, "running zellij");

        let output = cmd.output().await.with_context(|| format!("Failed to execute zellij {:?}", args))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("zellij command failed (exit {:?}): {}", output.status, stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute a zellij action subcommand.
    async fn run_action(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = self.build_cmd(&["action"]);
        cmd.args(args);

        tracing::debug!(?args, "running zellij action");

        let output = cmd.output().await.with_context(|| format!("Failed to execute zellij action {:?}", args))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("zellij action failed (exit {:?}): {}", output.status, stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// List all panes in the current session and return their IDs.
    #[allow(dead_code)]
    async fn list_panes(&self) -> anyhow::Result<Vec<String>> {
        let output = self.run_action(&["list-panes"]).await?;
        
        // Parse pane IDs from zellij's list-panes output
        // Format varies but typically includes "id: <number>" or similar
        let mut panes = Vec::new();
        for line in output.lines() {
            if let Some(id) = line.split(':').next().and_then(|s| s.trim().parse::<u32>().ok()) {
                panes.push(format!("{}", id));
            }
        }

        Ok(panes)
    }

    /// Get the first available pane ID, or None if no panes exist.
    #[allow(dead_code)]
    async fn get_first_pane_id(&self) -> anyhow::Result<Option<String>> {
        let panes = self.list_panes().await?;
        Ok(panes.into_iter().next())
    }
}

impl Default for ZellijBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TerminalBackend for ZellijBackend {
    fn name(&self) -> &str {
        "zellij"
    }

    fn is_available() -> bool {
        which::which("zellij").is_ok()
    }

    async fn spawn(
        &self,
        session_name: &str,
        working_dir: &Path,
    ) -> anyhow::Result<SpawnResult> {
        // Start a new zellij server session with the given command
        let mut cmd = Command::new("zellij");
        
        if let Some(ref prefix) = self.session_prefix {
            cmd.arg("-s").arg(prefix);
        } else {
            cmd.arg("-s").arg(session_name);
        }
        
        // Start in detached mode with a shell in the working directory
        cmd.arg("server")
           .arg("start");

        tracing::info!(session = %session_name, working_dir = ?working_dir, "starting zellij server");

        let output = cmd.output().await.with_context(|| "Failed to start zellij server")?;

        if !output.status.success() {
            // Server might already be running, which is fine
            if !String::from_utf8_lossy(&output.stderr).contains("already started") {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                tracing::warn!(stderr = %stderr, "zellij server start may have failed (non-fatal)");
            }
        }

        // Create a new pane in the session (result not needed for zellij)
        let _spawn_result = self.run_action(&["new-pane"]).await;
        
        Ok(SpawnResult {
            pane_id: None, // Zellij doesn't return pane IDs on creation via CLI
            session_name: session_name.to_string(),
        })
    }

    async fn send_text(&self, _session_name: &str, input: &str) -> anyhow::Result<()> {
        // Send text to the focused pane using write-chars (sends raw bytes)
        self.run_action(&["write-chars", input]).await?;
        
        Ok(())
    }

    async fn read_output(
        &self,
        _session_name: &str,
        lines: usize,
    ) -> anyhow::Result<String> {
        // Dump the screen content (viewport + optionally scrollback)
        let output = self.run_action(&["dump-screen"]).await?;
        
        // If we need only last N lines, truncate here
        if lines > 0 && output.lines().count() > lines {
            let last_lines: Vec<&str> = output.lines().rev().take(lines).collect();
            Ok(last_lines.into_iter().rev().collect::<Vec<_>>().join("\n"))
        } else {
            Ok(output)
        }
    }

    async fn kill_session(&self, session_name: &str) -> anyhow::Result<()> {
        // Kill the entire zellij session
        self.run_action(&["detach"]).await?;
        
        // Note: Zellij doesn't have a direct "kill session" CLI command.
        // The server continues running but the client detaches.
        // To fully kill, you'd need to send SIGTERM to the server process.
        
        tracing::info!(session = %session_name, "detached zellij session");
        Ok(())
    }

    async fn split_pane_horizontal(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        let mut args = vec!["split", "v"]; // v = vertical split (top/bottom)
        
        if let Some(dir) = working_dir {
            args.extend(["--cwd", dir.to_str().unwrap_or(".")]);
        }

        tracing::info!(session = %session_name, "splitting zellij pane horizontally");
        
        self.run_action(&args).await?;
        
        // Zellij doesn't return pane IDs via CLI, so we just return a placeholder
        Ok("zellij-split".to_string())
    }

    async fn split_pane_vertical(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        let mut args = vec!["split", "h"]; // h = horizontal split (left/right)
        
        if let Some(dir) = working_dir {
            args.extend(["--cwd", dir.to_str().unwrap_or(".")]);
        }

        tracing::info!(session = %session_name, "splitting zellij pane vertically");
        
        self.run_action(&args).await?;
        
        // Zellij doesn't return pane IDs via CLI, so we just return a placeholder
        Ok("zellij-split".to_string())
    }
}
