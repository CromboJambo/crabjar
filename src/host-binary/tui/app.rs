//! App state machine for the conversational TUI.
//!
//! Manages conversation history, agent loop lifecycle, and guard interaction.

use super::input;
use crate::session::{Session, SessionStore};
use crabjar_host_agent::{AgentLoop, LoopResult};
use crabjar_host_core::{EventBus, MetricsCollector};
use ratatui::Frame;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Current state of the TUI app.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Ready for user input
    Idle,
    /// Agent loop is running
    Running,
    /// Waiting for guard approval
    AwaitingApproval(String),
    /// Error occurred
    Error(String),
}

/// Message types in the conversation.
#[derive(Debug, Clone)]
pub enum Message {
    User { text: String },
    Agent { text: String },
    ToolCall { name: String, args: String, result: String },
    Guard { action: String, pending: bool },
}

/// The main application state.
pub struct App {
    /// Current UI state
    pub state: AppState,
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Input buffer (user is typing)
    pub input_buffer: String,
    /// Scroll position for message history
    pub scroll_offset: usize,
    /// Session store for persistence
    pub session_store: Option<SessionStore>,
    /// Current session ID
    pub current_session_id: Option<String>,
}

impl App {
    /// Create a new app instance.
    pub fn new(
        initial_objective: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = Self {
            state: AppState::Idle,
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            session_store: None,
            current_session_id: None,
        };

        // Initialize session store if data directory exists
        let data_dir = dirs::config_dir()
            .map(|d| d.join("crabjar").join("sessions"))
            .unwrap_or_else(|| PathBuf::from("/tmp/crabjar/sessions"));

        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            tracing::warn!("Failed to create session directory: {}", e);
        } else {
            app.session_store = Some(SessionStore::new(data_dir));
        }

        // Load or create session
        if let Some(sid) = session_id {
            if let Some(ref store) = app.session_store {
                if let Ok(session) = store.load(sid) {
                    app.current_session_id = Some(sid.to_string());
                    for msg in &session.messages {
                        app.messages.push(msg.clone());
                    }
                }
            }
        }

        // If no session loaded, create a new one
        if app.current_session_id.is_none() {
            if let Some(ref store) = app.session_store {
                let sid = store.create()?;
                app.current_session_id = Some(sid);
            }
        }

        // Add initial objective if provided
        if let Some(obj) = initial_objective {
            app.messages.push(Message::User { text: obj.to_string() });
        }

        Ok(app)
    }

    /// Handle a crossterm input event.
    pub fn handle_input(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Result<Option<input::Action>, Box<dyn std::error::Error>> {
        match event {
            crossterm::event::Event::Key(key) => {
                use crossterm::event::KeyCode;

                match key.code {
                    // Enter: submit input
                    KeyCode::Enter if !key.modifiers.is_empty() => {
                        // Shift+Enter = newline (handled by terminal)
                        Ok(None)
                    }
                    KeyCode::Enter => {
                        let text = self.input_buffer.clone();
                        self.input_buffer.clear();
                        if text.trim().is_empty() {
                            return Ok(None);
                        }
                        Ok(Some(input::Action::Submit(text)))
                    }
                    // Ctrl+C: quit
                    KeyCode::Char('c') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                        Ok(Some(input::Action::Quit))
                    }
                    // Arrow keys for scrolling when not in input mode
                    KeyCode::Up if self.state != AppState::Idle && !self.input_buffer.is_empty() => {
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                        Ok(None)
                    }
                    KeyCode::Down if self.state != AppState::Idle && !self.input_buffer.is_empty() => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        Ok(None)
                    }
                    // Backspace in input buffer
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                        Ok(None)
                    }
                    // Regular characters go into input buffer
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Render the app to the terminal.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Title bar
                Constraint::Min(0),     // Message area (grows)
                Constraint::Length(3),  // Status bar
                Constraint::Length(1),  // Input line
            ])
            .split(frame.area());

        // Title bar
        let title = match &self.state {
            AppState::Idle => "CrabJar Agent — Ready",
            AppState::Running => "CrabJar Agent — Running...",
            AppState::AwaitingApproval(_) => "CrabJar Agent — Awaiting Approval",
            AppState::Error(ref e) => format!("CrabJar Agent — Error: {}", e).as_str(),
        };

        let title_widget = Paragraph::new(title)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" CrabJar "),
            )
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        frame.render_widget(title_widget, chunks[0]);

        // Message area with scrollback
        let visible_messages: Vec<&Message> = self.messages.iter().rev().take(20).collect();
        let mut lines: Vec<Line<'_>> = Vec::new();

        for msg in visible_messages {
            match msg {
                Message::User { text } => {
                    lines.push(Line::from(vec![
                        Span::raw(" > "),
                        Span::styled(text.clone(), Style::default().fg(Color::Yellow)),
                    ]));
                }
                Message::Agent { text } => {
                    // Truncate long agent responses for display
                    let display = if text.len() > 200 {
                        format!("{}...", &text[..197])
                    } else {
                        text.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(display, Style::default().fg(Color::White)),
                    ]));
                }
                Message::ToolCall { name, args, result } => {
                    let display_result = if result.len() > 100 {
                        format!("{}...", &result[..97])
                    } else {
                        result.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("[tool: {}]", name), Style::default().fg(Color::Blue)),
                        Span::raw(" "),
                        Span::styled(args, Style::default().fg(Color::Magenta)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::raw("    → "),
                        Span::styled(display_result, Style::default().fg(Color::Green)),
                    ]));
                }
                Message::Guard { action, pending } => {
                    let color = if *pending { Color::Red } else { Color::Green };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("[guard: {}]", action),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
        }

        let message_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Conversation "));

        frame.render_widget(message_widget, chunks[1]);

        // Status bar
        let status_text = match &self.state {
            AppState::Idle => " Enter your request below ",
            AppState::Running => " Agent is working... ",
            AppState::AwaitingApproval(ref action) => format!(" Guard pending: {} [approve/reject] ", action).as_str(),
            AppState::Error(_) => " Error occurred ",
        };

        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(status_widget, chunks[2]);

        // Input line
        let input_widget = Paragraph::new(self.input_buffer.clone())
            .block(Block::default().borders(Borders::ALL).title(" Input "));

        frame.render_widget(input_widget, chunks[3]);
    }

    /// Set the current app state.
    pub fn set_state(&mut self, state: AppState) {
        match &state {
            AppState::Running => {
                self.messages.push(Message::Agent { text: "Starting agent loop...".to_string() });
            }
            AppState::Idle => {
                self.messages.push(Message::Agent { text: "Ready for next request.".to_string() });
            }
            _ => {}
        }
        self.state = state;
    }

    /// Run the agent loop with the given objective.
    pub async fn run_agent_loop(
        &mut self,
        objective: &str,
        tx: &mpsc::Sender<AppState>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize agent loop components
        let bus = std::sync::Arc::new(EventBus::new(16));
        let metrics = MetricsCollector::new();

        let mut loop_engine = AgentLoop::new(bus, metrics);
        loop_engine.start(objective);

        self.messages.push(Message::Agent { text: format!("Starting: {}", objective) });

        // Run the loop with iteration tracking
        let max_iterations = 50;
        for i in 1..=max_iterations {
            tx.send(AppState::Running).await?;

            match loop_engine.tick().await {
                Ok(result) => match result {
                    LoopResult::IterationComplete { confidence, tasks_completed } => {
                        self.messages.push(Message::Agent {
                            text: format!(
                                "Iteration {}: confidence={:.0}%, tasks={}",
                                i, confidence * 100.0, tasks_completed
                            ),
                        });

                        // Check if we should continue based on confidence
                        if confidence >= 0.85 {
                            self.messages.push(Message::Agent { text: "Sufficient confidence reached.".to_string() });
                            tx.send(AppState::Idle).await?;
                            break;
                        }
                    }
                    LoopResult::Completed { .. } => {
                        self.messages.push(Message::Agent { text: "Task completed successfully.".to_string() });
                        tx.send(AppState::Idle).await?;
                        break;
                    }
                    LoopResult::Failed { reason, .. } => {
                        self.messages.push(Message::Agent { text: format!("Failed: {}", reason) });
                        tx.send(AppState::Error(reason)).await?;
                        break;
                    }
                },
                Err(e) => {
                    self.messages.push(Message::Agent { text: format!("Loop error: {}", e) });
                    tx.send(AppState::Error(e.to_string())).await?;
                    break;
                }
            }

            // Small delay between iterations for readability
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Save to session if we have a store
        if let Some(ref store) = self.session_store {
            if let Some(ref sid) = self.current_session_id {
                store.save(sid, &self.messages)?;
            }
        }

        Ok(())
    }
}
