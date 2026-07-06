//! # crabjar-terminal
//!
//! Terminal multiplexer integration for agent harness.
//! Provides a unified API for spawning, controlling, and recording terminal sessions
//! via wezterm (primary) or zellij (fallback).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    TerminalManager                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
//! │  │ WeztermBackend│  │ ZellijBackend│  │ AsciinemaRecorder│  │
//! │  │ (primary)    │  │ (fallback)   │  │ (v2 format)      │  │
//! │  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
//! └─────────┼─────────────────┼────────────────────┼────────────┘
//!           │                 │                    │
//!           ▼                 ▼                    ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   TerminalSession                          │
//! │  • spawn() — start detached session                        │
//! │  • send()  — send text/keys to pane                        │
//! │  • read()  — read terminal output                          │
//! │  • snapshot() — capture screen/buffer state                │
//! │  • record() — start asciinema v2 recording                 │
//! │  • stop()  — kill session                                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crabjar_terminal::{TerminalManager, TerminalBackend};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut manager = TerminalManager::new();
//!     
//!     // Create a session with auto-detection (wezterm > zellij)
//!     let working_dir = PathBuf::from("/home/user/project");
//!     let mut session = manager.create_session("agent-work", working_dir)?;
//!     session.spawn().await?;
//!     
//!     // Send commands and read output
//!     session.send("cargo check\n").await?;
//!     let output = session.read(10).await?;
//!     
//!     // Snapshot terminal state
//!     let snap = session.snapshot().await?;
//!     println!("Terminal has {} lines", snap.lines.len());
//!     
//!     // Record session as asciicast v2
//!     let record_path = std::path::PathBuf::from("/tmp/session.cast");
//!     let _recorded = session.record(&record_path).await?;
//!     
//!     session.stop().await?;
//!     Ok(())
//! }
//! ```

pub mod backend;
mod recording;
mod wezterm;
mod zellij;

use std::path::{Path, PathBuf};

pub use backend::*;
pub use recording::*;
pub use wezterm::WeztermBackend;
pub use zellij::ZellijBackend;

/// Terminal session state snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    /// Session name
    pub session_name: String,
    /// Backend used
    pub backend: String,
    /// Current pane ID (if applicable)
    pub pane_id: Option<String>,
    /// Terminal buffer lines (last N lines)
    pub lines: Vec<String>,
    /// Cursor position (row, col) or None if unavailable
    pub cursor: Option<(usize, usize)>,
    /// Working directory of the session
    pub working_dir: PathBuf,
    /// Timestamp of snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Terminal session — wraps a backend with lifecycle management
#[derive(Debug)]
pub struct TerminalSession {
    backend: Box<dyn TerminalBackend + Send + Sync>,
    session_name: String,
    working_dir: PathBuf,
    pane_id: Option<String>,
    recorder: Option<AsciinemaRecorder>,
}

impl TerminalSession {
    /// Create a new terminal session with the given backend
    pub fn new(backend: Box<dyn TerminalBackend + Send + Sync>, name: &str, working_dir: PathBuf) -> Self {
        Self {
            backend,
            session_name: name.to_string(),
            working_dir,
            pane_id: None,
            recorder: None,
        }
    }

    /// Spawn a new detached terminal session
    pub async fn spawn(&mut self) -> anyhow::Result<()> {
        tracing::info!(session = %self.session_name, backend = ?self.backend.name(), "spawning terminal session");
        
        let result = self.backend.spawn(
            &self.session_name,
            &self.working_dir,
        ).await?;
        
        if let Some(pane) = &result.pane_id {
            self.pane_id = Some(pane.clone());
        }
        
        Ok(())
    }

    /// Send text/keys to the terminal session
    pub async fn send(&self, input: &str) -> anyhow::Result<()> {
        tracing::debug!(session = %self.session_name, chars = input.len(), "sending input");
        self.backend.send_text(&self.session_name, input).await
    }

    /// Read the last N lines of terminal output
    pub async fn read(&self, lines: usize) -> anyhow::Result<String> {
        tracing::debug!(session = %self.session_name, lines, "reading terminal output");
        self.backend.read_output(&self.session_name, lines).await
    }

    /// Capture a snapshot of the current terminal state
    pub async fn snapshot(&self) -> anyhow::Result<Snapshot> {
        tracing::debug!(session = %self.session_name, "capturing terminal snapshot");
        
        let output = self.backend.read_output(&self.session_name, 100).await?;
        let line_vec: Vec<String> = output.lines().map(|l| l.to_string()).collect();
        
        Ok(Snapshot {
            session_name: self.session_name.clone(),
            backend: self.backend.name().to_string(),
            pane_id: self.pane_id.clone(),
            lines: line_vec,
            cursor: None, // Backend-specific if available
            working_dir: self.working_dir.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Start recording this session as asciinema v2
    pub async fn record(&mut self, output_path: &Path) -> anyhow::Result<PathBuf> {
        tracing::info!(session = %self.session_name, path = ?output_path, "starting asciinema recording");
        
        let mut recorder = AsciinemaRecorder::new(
            &self.session_name,
            &self.backend.name(),
            output_path.to_path_buf(),
        );
        
        recorder.start()?;
        self.recorder = Some(recorder);
        
        Ok(output_path.to_path_buf())
    }

    /// Stop recording (if active) and terminate the session
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!(session = %self.session_name, "stopping terminal session");
        
        // Stop recorder first if active
        if let Some(ref mut recorder) = self.recorder.take() {
            recorder.stop()?;
        }
        
        self.backend.kill_session(&self.session_name).await
    }

    /// Get the session name
    pub fn name(&self) -> &str {
        &self.session_name
    }

    /// Check if recording is active
    pub fn is_recording(&self) -> bool {
        self.recorder.is_some()
    }
}

/// Terminal manager — tracks multiple terminal sessions
#[derive(Debug, Default)]
pub struct TerminalManager {
    sessions: std::collections::HashMap<String, TerminalSession>,
}

impl TerminalManager {
    /// Create a new terminal manager
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
        }
    }

    /// Create a new terminal session with auto-detection (wezterm > zellij)
    pub fn create_session(&mut self, name: &str, working_dir: PathBuf) -> anyhow::Result<TerminalSession> {
        // Auto-detect backend: prefer wezterm, fall back to zellij
        let backend_name = if WeztermBackend::is_available() {
            "wezterm"
        } else if ZellijBackend::is_available() {
            "zellij"
        } else {
            anyhow::bail!("No terminal multiplexer available (need wezterm or zellij)")
        };

        let backend = if WeztermBackend::is_available() {
            Box::new(WeztermBackend::new()) as Box<dyn TerminalBackend + Send + Sync>
        } else {
            Box::new(ZellijBackend::new()) as Box<dyn TerminalBackend + Send + Sync>
        };

        let session = TerminalSession::new(backend, name, working_dir);
        
        tracing::info!(session = %name, backend = backend_name, "created terminal session");
        Ok(session)
    }

    /// Get a terminal session by name
    pub fn get_session(&self, name: &str) -> Option<&TerminalSession> {
        self.sessions.get(name)
    }

    /// List all managed sessions
    pub fn list_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a session from tracking (doesn't kill the underlying process)
    pub fn remove_session(&mut self, name: &str) -> Option<TerminalSession> {
        tracing::info!(session = %name, "removed terminal session from manager");
        self.sessions.remove(name)
    }
}
