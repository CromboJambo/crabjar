//! Browser automation data models.

use serde::{Deserialize, Serialize};

/// Result of navigation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationResult {
    pub url: String,
    pub title: String,
    pub content: String,
}

/// Result of JavaScript evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub value: serde_json::Value,
}

/// Result of screenshot capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub data: Vec<u8>,
    pub format: String,
}
