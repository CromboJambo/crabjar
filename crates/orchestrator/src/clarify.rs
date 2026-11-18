//! P1-5: Interactive Clarify Tooling (Hermes API parity)
//!
//! Enables agents to ask users structured questions with choice-based responses.
//! Supports 1-5 questions per call, single-select, multi-select, and open-ended.

use serde::{Deserialize, Serialize};

/// A single question in a clarify form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyQuestion {
    /// The question text (options excluded — they go in `choices`).
    pub question: String,
    /// Optional choices to present as pickable rows.
    #[serde(default)]
    pub choices: Vec<String>,
    /// Whether multiple selections are allowed (only meaningful with choices).
    #[serde(default)]
    pub multi_select: bool,
}

/// Input payload for the clarify tool (Hermes-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyInput {
    /// The questions to ask (1-5 entries).
    pub questions: Vec<ClarifyQuestion>,
    /// Optional timeout in seconds (default: 300).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300
}

/// Response to a single question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyResponse {
    /// The original question text (echoed back).
    pub question: String,
    /// Status of this response.
    pub status: String, // "answered" | "skipped" | "unanswered"
    /// The user's response (null unless answered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_response: Option<String>,
}

/// Result returned to the agent after user responds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyResult {
    /// Responses in question order.
    pub responses: Vec<ClarifyResponse>,
    /// Overall outcome of the clarify call.
    pub outcome: String, // "submitted" | "cancelled" | "timed_out" | "undelivered"
}

/// SSE event payload sent to the TUI when a question is posed.
#[derive(Debug, Clone, Serialize)]
pub struct ClarifyEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: String,
    pub questions: Vec<ClarifyQuestion>,
}

/// Response payload sent back from TUI when user answers.
#[derive(Debug, Clone, Deserialize)]
pub struct ClarifyResponsePayload {
    pub id: String,
    pub responses: Vec<String>, // one per question, in order
}
