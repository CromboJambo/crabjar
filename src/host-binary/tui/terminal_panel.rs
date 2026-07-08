//! Terminal panel widget for displaying agent terminal sessions in the TUI.
//!
//! Wraps `crabjar-terminal` to provide a live terminal view within the ratatui UI.

use crabjar_terminal::{TerminalManager, TerminalSession};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for a terminal panel.
#[derive(Debug, Clone)]
pub struct TerminalPanelConfig {
    /// Session name (used to identify the session)
    pub session_name: String,
    /// Whether to show the title bar
    pub show_title: bool,
}

impl Default for TerminalPanelConfig {
    fn default() -> Self {
        Self {
            session_name: "agent".to_string(),
            show_title: true,
        }
    }
}

/// A terminal panel that displays a crabjar-terminal session.
#[allow(dead_code)]
pub struct TerminalPanel {
    /// Configuration for this panel
    config: TerminalPanelConfig,
    /// The underlying terminal session (wrapped in Arc<Mutex<>> for interior mutability)
    session: Option<Arc<Mutex<TerminalSession>>>,
    /// Buffer for displaying terminal output
    output_buffer: Vec<String>,
}

#[allow(dead_code)]
impl TerminalPanel {
    /// Create a new terminal panel with the given configuration.
    /// Returns `None` if no terminal backend is available (wezterm/zellij not installed).
    pub async fn try_new(config: TerminalPanelConfig) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let mut manager = TerminalManager::new();

        // Try to create and spawn a session; fail gracefully if no backend available
        match manager.create_session(&config.session_name, PathBuf::from("/tmp")) {
            Ok(mut session) => {
                if session.spawn().await.is_ok() {
                    Ok(Some(Self {
                        config,
                        session: Some(Arc::new(Mutex::new(session))),
                        output_buffer: Vec::new(),
                    }))
                } else {
                    tracing::warn!("Failed to spawn terminal session");
                    Ok(None)
                }
            }
            Err(e) => {
                tracing::info!("No terminal backend available (wezterm/zellij not installed): {}", e);
                Ok(None)
            }
        }
    }

    /// Create a new terminal panel (convenience wrapper — panics if unavailable).
    #[deprecated(since = "0.1.0", note = "Use `try_new` instead for graceful degradation")]
    pub async fn new(config: TerminalPanelConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Self::try_new(config).await?.ok_or_else(|| "No terminal backend available".into())
    }

    /// Send text input to the terminal session.
    pub async fn send_input(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref session) = self.session {
            let sess = session.lock().await;
            sess.send(text).await?;
        }
        Ok(())
    }

    /// Update the output buffer with fresh data from the terminal.
    pub async fn update_output(&mut self, max_lines: usize) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref session) = self.session {
            let sess = session.lock().await;
            // Use snapshot() to get current terminal state
            let snap = sess.snapshot().await?;
            
            for line in &snap.lines {
                self.output_buffer.push(line.clone());
            }
            
            // Trim buffer to max_lines
            while self.output_buffer.len() > max_lines {
                self.output_buffer.remove(0);
            }
        }
        Ok(())
    }

    /// Render the terminal panel.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = if self.config.show_title {
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Terminal [{}] ", self.config.session_name))
        } else {
            Block::default().borders(Borders::ALL)
        };

        // Create lines from the output buffer
        let lines: Vec<Line> = self.output_buffer.iter()
            .map(|line| Line::from(Span::raw(line.clone())))
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(block);

        frame.render_widget(paragraph, area);
    }

    /// Get the current output as a string.
    pub fn get_output(&self) -> String {
        self.output_buffer.join("\n")
    }
}
