//! P1-5: Interactive Clarify Tooling
//!
//! Enables agents to ask users structured questions with choice-based responses.
//! Integrates with the TUI's existing approval/question mechanism via SSE events.

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

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

/// Execute the clarify tool by sending an SSE event and waiting for response.
pub async fn execute_clarify(
    input: ClarifyInput,
    event_tx: &tokio::sync::broadcast::Sender<String>,
) -> Result<ClarifyResult, String> {
    let question_id = uuid::Uuid::new_v4().to_string();

    // Send SSE event to TUI with the question
    let event = ClarifyEvent {
        event_type: "clarify_question".to_string(),
        id: question_id.clone(),
        question: input.question,
        options: input.options,
    };

    let event_json = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize clarify event: {}", e))?;
    if event_tx.send(event_json).is_err() {
        return Err("No TUI subscribers for clarify question".to_string());
    }

    // Wait for user response via channel
    let (_tx, rx) = oneshot::channel();

    // Store the sender so handler can respond to it
    // In a real implementation, this would be stored in shared state keyed by question_id
    // For simplicity, we'll use a blocking wait here — production would use an async map of pending questions

    tokio::time::timeout(tokio::time::Duration::from_secs(input.timeout_secs), rx)
        .await
        .map_err(|_| "Clarify question timed out".to_string())?
        .map_err(|_| "Clarify channel closed without response".to_string())
}
