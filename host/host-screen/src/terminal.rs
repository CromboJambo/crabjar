//! # host-terminal
//!
//! Shared terminal integration via wezterm or zellij.
//! Provides a unified API for terminal multiplexing over WebSocket.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Terminal multiplexer to use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminalBackend {
    /// Wezterm terminal multiplexer
    WezTerm,
    /// Zellij terminal multiplexer
    Zellij,
}

/// Terminal session
#[derive(Debug, Clone)]
pub struct TerminalSession {
    /// Backend
    pub backend: TerminalBackend,
    /// Session name
    pub session_name: String,
    /// Working directory
    pub working_dir: Option<std::path::PathBuf>,
}

impl TerminalSession {
    /// Create a new terminal session
    pub fn new(backend: TerminalBackend, session_name: &str) -> Self {
        Self {
            backend,
            session_name: session_name.to_string(),
            working_dir: None,
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Start the terminal session
    pub async fn start(&self) -> Result<()> {
        match &self.backend {
            TerminalBackend::WezTerm => {
                tracing::info!(session = %self.session_name, "starting wezterm session");
                // TODO: Implement wezterm session management
            }
            TerminalBackend::Zellij => {
                tracing::info!(session = %self.session_name, "starting zellij session");
                // TODO: Implement zellij session management
            }
        }
        Ok(())
    }

    /// Stop the terminal session
    pub async fn stop(&self) -> Result<()> {
        tracing::info!(session = %self.session_name, "stopping terminal session");
        Ok(())
    }
}

/// Terminal manager
#[derive(Debug, Clone, Default)]
pub struct TerminalManager {
    sessions: std::collections::HashMap<String, TerminalSession>,
}

impl TerminalManager {
    /// Create a new TerminalManager
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new,
        }
    }

    /// Create a new terminal session
    pub fn create_session(
        &mut self,
        name: &str,
        backend: TerminalBackend,
    ) -> TerminalSession {
        let session = TerminalSession::new(backend, name);
        self.sessions.insert(name.to_string(), session.clone());
        session
    }

    /// Get a terminal session by name
    pub fn get_session(&self, name: &str) -> Option<&TerminalSession> {
        self.sessions.get(name)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_session_creation() {
        let session = TerminalSession::new(TerminalBackend::WezTerm, "test");
        assert_eq!(session.session_name, "test");
    }
}
