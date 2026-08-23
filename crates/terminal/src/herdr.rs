//! Herdr backend implementation (ADR-002 spike).
//!
//! Drives a running Herdr server over its CLI/socket API. Herdr is the
//! execution substrate: it owns workspaces, panes, and agent lifecycle.
//! This backend is a thin translation layer from the `TerminalBackend`
//! trait onto `herdr` subcommands, following the `ProductAdapter`
//! pattern — no core changes.
//!
//! ## Session mapping
//!
//! A crabjar "session" maps to a Herdr **workspace**:
//!
//! - `spawn(name, dir)` → `herdr workspace create --cwd <dir> --label <name>`
//!   The root pane of the new workspace is the session's pane.
//! - `send_text` → `herdr pane send-text <pane> <text>`
//! - `read_output` → `herdr pane read <pane> --source visible --lines N`
//!   (the default `recent` source returned empty on 0.8.2; `visible` is
//!   the reliable source for spike purposes)
//! - `kill_session` → `herdr workspace close <workspace>`
//! - `split_pane_*` → `herdr pane split <pane> --direction down|right`
//!
//! ## Connection state
//!
//! Unlike the wezterm/zellij backends, Herdr is a long-lived server. The
//! backend therefore carries connection state: the `herdr` binary path
//! and the server socket path. `is_available_async` means "binary on
//! PATH *and* server reachable", not just "binary exists".
//!
//! ## Spike caveats (intentionally left rough)
//!
//! - `send_text`/`read_output` target the session's root pane only;
//!   panes created via splits are not yet tracked per-session.
//! - No reconnection logic: if the server dies mid-session, commands fail
//!   and the caller must respawn.
//! - Agent state (working/blocked/idle) is exposed via `agent_status()`
//!   but not yet wired into `TerminalSession`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use tokio::process::Command;

use crate::backend::{SpawnResult, TerminalBackend};

/// Herdr socket API backend for terminal session management.
///
/// `Clone` shares the session map (`Arc<Mutex<..>>`) so a caller can keep
/// a handle for split/agent-status calls while `TerminalSession` owns
/// another clone of the same backend state.
#[derive(Debug, Clone)]
pub struct HerdrBackend {
    /// Path to the `herdr` binary (usually just "herdr" on PATH).
    binary: String,
    /// Server socket path, e.g. `~/.config/herdr/herdr.sock`.
    ///
    /// Reserved connection state: the current herdr CLI resolves the
    /// socket from its own config, so this is not yet passed to the
    /// binary. It exists so the backend can later target a specific
    /// server (tailnet host, alternate socket) without a trait change.
    socket_path: Option<String>,
    /// session_name → (workspace_id, root_pane_id)
    ///
    /// `Arc<Mutex<..>>` because `TerminalBackend` methods take `&self`
    /// (spawn/kill must still record and release state) and `Clone` must
    /// share the map across backend handles.
    sessions: Arc<Mutex<HashMap<String, (String, String)>>>,
}

impl HerdrBackend {
    /// Create a backend targeting the default local Herdr server.
    pub fn new() -> Self {
        Self {
            binary: "herdr".to_string(),
            socket_path: None,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Target a specific `herdr` binary path.
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Explicit server socket path (reserved; see field docs).
    pub fn with_socket(mut self, socket_path: impl Into<String>) -> Self {
        self.socket_path = Some(socket_path.into());
        self
    }

    /// Run a herdr subcommand, return the parsed `result` value.
    ///
    /// All herdr CLI commands print a JSON envelope:
    /// `{"id": "cli:...", "result": {...}}`. Commands that print nothing
    /// on success (e.g. `pane send-text`) yield `Value::Null`.
    async fn run_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(args);

        let output = cmd
            .output()
            .await
            .with_context(|| format!("failed to execute {}", self.binary))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("herdr {} failed: {}", args.join(" "), stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let value: serde_json::Value = serde_json::from_str(&stdout).with_context(|| {
            format!(
                "herdr {} returned non-JSON output: {}",
                args.join(" "),
                &stdout[..stdout.len().min(200)]
            )
        })?;

        // Envelope: {"id": "...", "result": {...}}
        match value.get("result") {
            Some(result) => Ok(result.clone()),
            None => Ok(value),
        }
    }

    /// Look up (workspace_id, pane_id) for a session.
    fn session_pane(&self, session_name: &str) -> anyhow::Result<(String, String)> {
        let map = self
            .sessions
            .lock()
            .map_err(|e| anyhow::anyhow!("herdr session map poisoned: {e}"))?;
        map.get(session_name)
            .cloned()
            .with_context(|| format!("no herdr workspace tracked for session '{session_name}'"))
    }

    /// Run a herdr subcommand, return stdout verbatim.
    ///
    /// Most herdr CLI commands print a JSON envelope, but `pane read`
    /// prints the pane's raw terminal text — callers for that command
    /// must not parse JSON.
    async fn run_raw(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(args);

        let output = cmd
            .output()
            .await
            .with_context(|| format!("failed to execute {}", self.binary))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("herdr {} failed: {}", args.join(" "), stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Check whether a Herdr server is reachable.
    async fn server_reachable(&self) -> bool {
        let Ok(output) = Command::new(&self.binary).arg("status").output().await else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("status: running")
    }

    /// Async availability: binary on PATH **and** server reachable.
    ///
    /// The trait's sync `is_available` can only do the binary check;
    /// callers that need certainty should await this.
    pub async fn is_available_async(&self) -> bool {
        which::which(&self.binary).is_ok() && self.server_reachable().await
    }

    /// Agent status for a session's root pane, if herdr reports one.
    ///
    /// Returns `Some("working" | "idle" | "blocked" | "done" | "unknown")`
    /// when a known agent is running in the pane, `None` otherwise.
    pub async fn agent_status(&self, session_name: &str) -> anyhow::Result<Option<String>> {
        let (_, pane_id) = self.session_pane(session_name)?;

        let result = self.run_json(&["pane", "get", &pane_id]).await?;
        let status = result
            .get("agent_status")
            .and_then(|v| v.as_str())
            .map(String::from);

        // `unknown` means "no agent detected in this pane" — treat as None.
        Ok(status.filter(|s| s != "unknown"))
    }
}

impl Default for HerdrBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TerminalBackend for HerdrBackend {
    fn name(&self) -> &str {
        "herdr"
    }

    fn is_available() -> bool {
        which::which("herdr").is_ok()
    }

    async fn spawn(&self, session_name: &str, working_dir: &Path) -> anyhow::Result<SpawnResult> {
        let result = self
            .run_json(&[
                "workspace",
                "create",
                "--cwd",
                working_dir.as_os_str().to_str().unwrap_or("."),
                "--label",
                session_name,
                "--no-focus",
            ])
            .await?;

        let workspace_id = result
            .get("workspace")
            .and_then(|w| w.get("workspace_id"))
            .and_then(|v| v.as_str())
            .with_context(|| "herdr workspace create: no workspace_id in result")?
            .to_string();

        let pane_id = result
            .get("root_pane")
            .and_then(|p| p.get("pane_id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        tracing::info!(
            session = %session_name,
            workspace = %workspace_id,
            pane = ?pane_id,
            "spawned herdr workspace"
        );

        if let Some(pane) = &pane_id {
            let mut map = self
                .sessions
                .lock()
                .map_err(|e| anyhow::anyhow!("herdr session map poisoned: {e}"))?;
            map.insert(
                session_name.to_string(),
                (workspace_id.clone(), pane.clone()),
            );
        }

        Ok(SpawnResult {
            pane_id,
            session_name: session_name.to_string(),
        })
    }

    async fn send_text(&self, session_name: &str, input: &str) -> anyhow::Result<()> {
        let (_, pane_id) = self.session_pane(session_name)?;

        // send-text prints nothing on success; run_json returns Null.
        self.run_json(&["pane", "send-text", &pane_id, input])
            .await?;
        Ok(())
    }

    async fn read_output(&self, session_name: &str, lines: usize) -> anyhow::Result<String> {
        let (_, pane_id) = self.session_pane(session_name)?;

        // NOTE: `pane read` prints raw terminal text, not a JSON envelope.
        let text = self
            .run_raw(&[
                "pane",
                "read",
                &pane_id,
                "--source",
                "visible",
                "--lines",
                &lines.to_string(),
            ])
            .await?;

        // `--lines` caps the snapshot; trim to the last N lines for the
        // trait contract ("last N lines").
        let trimmed: Vec<&str> = text.lines().rev().take(lines).collect();
        let mut out: Vec<String> = trimmed.into_iter().rev().map(String::from).collect();
        if !out.is_empty() {
            out.push(String::new());
        }
        Ok(out.join("\n"))
    }

    async fn kill_session(&self, session_name: &str) -> anyhow::Result<()> {
        let (workspace_id, _) = self.session_pane(session_name)?;

        self.run_json(&["workspace", "close", &workspace_id])
            .await?;

        let mut map = self
            .sessions
            .lock()
            .map_err(|e| anyhow::anyhow!("herdr session map poisoned: {e}"))?;
        map.remove(session_name);

        tracing::info!(
            session = %session_name,
            workspace = %workspace_id,
            "closed herdr workspace"
        );
        Ok(())
    }

    async fn split_pane_horizontal(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        self.split_pane(session_name, working_dir, "down").await
    }

    async fn split_pane_vertical(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
    ) -> anyhow::Result<String> {
        self.split_pane(session_name, working_dir, "right").await
    }
}

impl HerdrBackend {
    /// Shared split logic: `herdr pane split <pane> --direction <dir>`.
    async fn split_pane(
        &self,
        session_name: &str,
        working_dir: Option<&Path>,
        direction: &str,
    ) -> anyhow::Result<String> {
        let (_, pane_id) = self.session_pane(session_name)?;

        let mut args: Vec<String> = vec![
            "pane".into(),
            "split".into(),
            pane_id,
            "--direction".into(),
            direction.into(),
        ];
        if let Some(dir) = working_dir {
            args.push("--cwd".into());
            args.push(dir.to_string_lossy().into_owned());
        }

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let result = self.run_json(&arg_refs).await?;

        let new_pane = result
            .get("pane")
            .and_then(|p| p.get("pane_id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(new_pane.unwrap_or_else(|| "split".to_string()))
    }
}
