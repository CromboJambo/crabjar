//! App state machine for the conversational TUI.
//!
//! Manages conversation history, agent loop lifecycle, and guard interaction.

use super::input;
use super::session::SessionStore;
use super::terminal_panel::TerminalPanel;
use crabjar_guard::GuardDb;
use crabjar_host_agent::{AgentLoop, LoopResult};
use crabjar_host_core::EventBus;
use crabjar_host_observe::MetricsCollector;
use ratatui::Frame;
use std::borrow::Cow;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Current state of the TUI app.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Ready for user input
    Idle,
    /// Agent loop is running
    Running,
    /// Waiting for guard approval — holds pending entry ID and action description
    AwaitingApproval { id: String, action_desc: String },
    /// Error occurred
    Error(String),
}

/// Message types in the conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Terminal panel for displaying agent terminal sessions
    pub terminal_panel: Option<TerminalPanel>,
    /// Guard database for pending queue operations
    pub guard_db: Option<GuardDb>,
}

impl App {
    /// Create a new app instance.
    pub fn new(
        initial_objective: Option<&str>,
        session_id: Option<&str>,
        guard_db: Option<GuardDb>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = Self {
            state: AppState::Idle,
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            session_store: None,
            current_session_id: None,
            terminal_panel: None,
            guard_db,
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
        if let Some(sid) = session_id
            && let Some(ref store) = app.session_store
            && let Ok(session) = store.load(sid) {
            app.current_session_id = Some(sid.to_string());
            for msg in &session.messages {
                app.messages.push(msg.clone());
            }
        }

        // If no session loaded, create a new one
        if app.current_session_id.is_none()
            && let Some(ref store) = app.session_store {
            let sid = store.create()?;
            app.current_session_id = Some(sid);
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

                // Handle guard approval shortcuts when awaiting approval
                if let AppState::AwaitingApproval { .. } = self.state {
                    match key.code {
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            return Ok(Some(input::Action::ApprovePending));
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            return Ok(Some(input::Action::RejectPending));
                        }
                        _ => {} // fall through to normal input handling below
                    }
                }

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

        // Split into left (terminal) and right (chat) panels
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Percentage(40),  // Terminal panel
                Constraint::Percentage(60),  // Chat panel
            ])
            .split(frame.area());

        // Render terminal panel on the left
        if let Some(ref panel) = self.terminal_panel {
            panel.render(frame, chunks[0]);
        } else {
            // Fallback: show a placeholder when no terminal is available
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Terminal [unavailable] ");
            frame.render_widget(block, chunks[0]);
        }

        // Split the right side into chat components
        let chat_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Title bar
                Constraint::Min(0),     // Message area (grows)
                Constraint::Length(3),  // Status bar
                Constraint::Length(1),  // Input line
            ])
            .split(chunks[1]);

        // Title bar
        let title = match &self.state {
            AppState::Idle => Cow::Borrowed("CrabJar Agent — Ready"),
            AppState::Running => Cow::Borrowed("CrabJar Agent — Running..."),
            AppState::AwaitingApproval { action_desc, .. } => {
                Cow::Owned(format!("CrabJar Agent — Awaiting Approval: {}", action_desc))
            }
            AppState::Error(e) => Cow::Owned(format!("CrabJar Agent — Error: {}", e)),
        };

        let title_widget = Paragraph::new(title.as_ref())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" CrabJar "),
            )
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        frame.render_widget(title_widget, chat_chunks[0]);

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

        frame.render_widget(message_widget, chat_chunks[1]);

        // Status bar
        let status_text: Cow<'_, str> = match &self.state {
            AppState::Idle => Cow::Borrowed(" Enter your request below "),
            AppState::Running => Cow::Borrowed(" Agent is working... "),
            AppState::AwaitingApproval { id: _, action_desc } => {
                Cow::Owned(format!(
                    " Guard pending: {} [a=approve / r=reject] ",
                    action_desc
                ))
            }
            AppState::Error(_) => Cow::Borrowed(" Error occurred "),
        };

        let status_widget = Paragraph::new(status_text.as_ref())
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(status_widget, chat_chunks[2]);

        // Input line
        let input_widget = Paragraph::new(self.input_buffer.clone())
            .block(Block::default().borders(Borders::ALL).title(" Input "));

        frame.render_widget(input_widget, chat_chunks[3]);
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

    /// Resolve a pending guard action by approving or rejecting it.
    pub fn resolve_pending(&mut self, approved: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Extract the pending entry ID from current state
        let (id, action_desc) = match &self.state {
            AppState::AwaitingApproval { id, action_desc } => (id.clone(), action_desc.clone()),
            _ => return Err("Not in AwaitingApproval state".into()),
        };

        // Resolve via GuardDb if available
        if let Some(ref db) = self.guard_db {
            db.resolve_pending_queue_entry(&id, approved)?;
        }

        // Update state and message
        let decision = if approved { "approved" } else { "rejected" };
        self.messages.push(Message::Guard {
            action: format!("{} (user {})", action_desc, decision),
            pending: false,
        });
        self.state = AppState::Idle;

        Ok(())
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

        let mut loop_engine = AgentLoop::new(bus, metrics)
            .with_scope(crabjar_guard::Scope::project("tui"));
        loop_engine.start(objective);

        self.messages.push(Message::Agent { text: format!("Starting: {}", objective) });

        // Run the loop with iteration tracking
        let max_iterations = 50;
        for i in 1..=max_iterations {
            tx.send(AppState::Running).await?;

            match loop_engine.tick().await {
                Ok(result) => match result {
                    LoopResult::IterationComplete { work_item_id: _, confidence, tasks_completed } => {
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
                    LoopResult::Completed { work_item_id: _ } => {
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

            // Check for pending guard actions and surface them to the user
            if let Some(ref db) = self.guard_db
                && let Ok(pending_entries) = db.read_pending_queue()
                && !pending_entries.is_empty() {
                // Surface the first pending entry as a guard message
                let entry = &pending_entries[0];
                let action_desc = format!(
                    "{} {} {}",
                    entry.command,
                    entry.args.join(" "),
                    if entry.reason.len() > 40 {
                        &entry.reason[..40]
                    } else {
                        &entry.reason
                    }
                );

                self.messages.push(Message::Guard {
                    action: format!("{} (pending review)", action_desc),
                    pending: true,
                });

                // Set state to awaiting approval — user must approve/reject before continuing
                tx.send(AppState::AwaitingApproval {
                    id: entry.id.clone(),
                    action_desc: action_desc.clone(),
                }).await?;

                // Break out of the loop — wait for user input via keyboard shortcuts
                break;
            }

            // Small delay between iterations for readability
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Save to session if we have a store
        if let Some(ref store) = self.session_store
            && let Some(ref sid) = self.current_session_id {
            store.save(sid, &self.messages)?;
        }

        Ok(())
    }
}
