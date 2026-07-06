//! Conversational TUI for CrabJar agent harness.
//!
//! Entry point: `run()` — launches a ratatui terminal with an interactive chat interface.
//! User types natural language requests at the bottom; the agent loop runs its ReAct cycle
//! and streams results back as they complete.

pub mod app;
pub mod input;
mod session;
pub mod terminal_panel;

use app::{App, AppState};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::DefaultTerminal;
use std::io;
use tokio::sync::mpsc;

/// Launch the conversational TUI.
pub async fn run(
    objective: Option<&str>,
    session_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal — use a wrapper to avoid move issues
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Run the TUI app (blocks until user quits)
    let result = run_app(&mut terminal, objective, session_id).await;

    // Restore terminal
    drop(terminal);
    terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    result.map_err(Into::into)
}

/// Core event loop: render → poll input → process → repeat.
async fn run_app(
    terminal: &mut DefaultTerminal,
    initial_objective: Option<&str>,
    session_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(64);

    // Build the app state
    let mut app = App::new(initial_objective, session_id)?;

    loop {
        terminal.draw(|frame| app.render(frame))?;

        // Poll for user input (non-blocking via short timeout)
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            let event = crossterm::event::read()?;
            if let Some(action) = app.handle_input(&event)? {
                match action {
                    input::Action::Submit(text) => {
                        tx.send(AppState::Running).await?;
                        // Run agent loop inline — AgentLoop contains rusqlite (RefCell) which isn't Send,
                        // so it can't be moved into tokio::spawn. The TUI already blocks on run_app,
                        // and the UI is in "Running" state during execution anyway.
                        if let Err(e) = app.run_agent_loop(&text, &tx).await {
                            let _ = tx.send(AppState::Error(format!("Agent error: {}", e))).await;
                        }
                    }
                    input::Action::Quit => break,
                }
            }
        }

        // Check for agent loop state changes (non-blocking)
        while let Ok(state) = rx.try_recv() {
            app.set_state(state);
        }
    }

    Ok(())
}
