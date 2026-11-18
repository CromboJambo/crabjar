//! P1-5: Interactive Clarify Tooling
//!
//! Enables agents to ask users structured questions with choice-based responses.
//! Integrates with the TUI's existing approval/question mechanism via SSE events.

use serde::{Deserialize, Serialize};

/// A single question option for the user to select from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub value: Option<String>,
}

/// Input payload for the clarify tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyInput {
    /// The question to ask the user.
    pub question: String,
    /// Optional choices to present (empty = free-text response).
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    /// Optional timeout in seconds (default: 300).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300
}

/// Result returned to the agent after user responds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyResult {
    /// Whether the user answered or timed out.
    pub responded: bool,
    /// The selected option index (if choice-based) or None for free-text.
    pub selection: Option<usize>,
    /// The selected option label (if any).
    pub selected_label: Option<String>,
    /// Free-text response from user (if no options or custom text provided).
    pub text_response: Option<String>,
}

/// SSE event payload sent to the TUI when a question is posed.
#[derive(Debug, Clone, Serialize)]
pub struct ClarifyEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: String,
    pub question: String,
    #[serde(default)]
    pub options: Vec<QuestionOption>,
}

/// Response payload sent back from TUI when user answers.
#[derive(Debug, Clone, Deserialize)]
pub struct ClarifyResponse {
    pub id: String,
    pub selection: Option<usize>,
    pub text_response: Option<String>,
}
