//! Rendering for the conversational TUI.
//!
//! Split from `app.rs` under the 500-LoC module governance rule: the
//! state machine lives in `app.rs`, the ratatui layout lives here.

use super::app::{App, AppState, Message};
use ratatui::Frame;
use std::borrow::Cow;

impl App {
    /// Render the app to the terminal.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        // Split into left (terminal), middle (habitat, when visible), right (chat)
        let constraints: Vec<Constraint> = if self.show_habitat {
            vec![
                Constraint::Percentage(35), // Terminal panel
                Constraint::Percentage(30), // Habitat panel
                Constraint::Percentage(35), // Chat panel
            ]
        } else {
            vec![
                Constraint::Percentage(40), // Terminal panel
                Constraint::Percentage(60), // Chat panel
            ]
        };
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(constraints)
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

        // Habitat panel (middle column, only when visible)
        let chat_area = if self.show_habitat {
            if let Some(ref panel) = self.habitat_panel {
                panel.render(frame, chunks[1]);
            } else {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Habitat [no store] ");
                frame.render_widget(block, chunks[1]);
            }
            chunks[2]
        } else {
            chunks[1]
        };

        // Split the right side into chat components
        let chat_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Message area (grows)
                Constraint::Length(3), // Status bar
                Constraint::Length(1), // Input line
            ])
            .split(chat_area);

        // Title bar
        let title = match &self.state {
            AppState::Idle => Cow::Borrowed("CrabJar Agent — Ready"),
            AppState::Running => Cow::Borrowed("CrabJar Agent — Running..."),
            AppState::AwaitingApproval { action_desc, .. } => Cow::Owned(format!(
                "CrabJar Agent — Awaiting Approval: {}",
                action_desc
            )),
            AppState::Error(e) => Cow::Owned(format!("CrabJar Agent — Error: {}", e)),
        };

        let title_widget = Paragraph::new(title.as_ref())
            .block(Block::default().borders(Borders::ALL).title(" CrabJar "))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

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
                        Span::styled(
                            format!("[tool: {}]", name),
                            Style::default().fg(Color::Blue),
                        ),
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
                    lines.push(Line::from(vec![Span::styled(
                        format!("[guard: {}]", action),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )]));
                }
            }
        }

        let message_widget = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Conversation "),
        );

        frame.render_widget(message_widget, chat_chunks[1]);

        // Status bar
        let status_text: Cow<'_, str> = match &self.state {
            AppState::Idle => {
                if self.show_habitat {
                    Cow::Borrowed(" [h] hide habitat   Enter your request below ")
                } else {
                    Cow::Borrowed(" [h] habitat   Enter your request below ")
                }
            }
            AppState::Running => Cow::Borrowed(" Agent is working... "),
            AppState::AwaitingApproval { id: _, action_desc } => Cow::Owned(format!(
                " Guard pending: {} [a=approve / r=reject] ",
                action_desc
            )),
            AppState::Error(_) => Cow::Borrowed(" Error occurred "),
        };

        let status_widget =
            Paragraph::new(status_text.as_ref()).style(Style::default().fg(Color::Yellow));

        frame.render_widget(status_widget, chat_chunks[2]);

        // Input line
        let input_widget = Paragraph::new(self.input_buffer.clone())
            .block(Block::default().borders(Borders::ALL).title(" Input "));

        frame.render_widget(input_widget, chat_chunks[3]);
    }
}
