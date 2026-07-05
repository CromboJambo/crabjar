//! Input handling for the conversational TUI.
//!
//! Defines input actions that can be triggered by user interaction.

/// Actions that can result from user input.
#[derive(Debug, Clone)]
pub enum Action {
    /// Submit text to the agent loop
    Submit(String),
    /// Quit the application
    Quit,
}
