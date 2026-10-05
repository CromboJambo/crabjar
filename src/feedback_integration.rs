/// User feedback integration: capture corrections, ratings, and style preferences.
///
/// Extends the guard approval flow with richer feedback signals that adapt
/// future agent behavior without requiring manual policy updates.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use serde_json::{json, Value};

/// Types of user feedback.
#[derive(Debug, Clone)]
pub enum FeedbackType {
    Approval,       // User approved an action
    Rejection,      // User rejected an action
    Correction,     // User said "do it differently"
    Rating,         // Post-completion quality rating (1-5)
    Preference,     // Style/behavior preference signal
}

/// Captures and stores user feedback for adaptive behavior.
pub struct FeedbackCollector {
    feedback_path: PathBuf,
}

impl FeedbackCollector {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let data_dir = PathBuf::from(home).join(".crabjar");
        fs::create_dir_all(&data_dir).ok();

        Self {
            feedback_path: data_dir.join("user-feedback.jsonl"),
        }
    }

    /// Record a feedback event.
    pub fn record(&self, task_id: &str, feedback_type: FeedbackType, details: Option<&Value>) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = json!({
            "type": match &feedback_type {
                FeedbackType::Approval => "approval",
                FeedbackType::Rejection => "rejection",
                FeedbackType::Correction => "correction",
                FeedbackType::Rating => "rating",
                FeedbackType::Preference => "preference",
            },
            "task_id": task_id,
            "timestamp": timestamp,
            "details": details.unwrap_or(&json!({})),
        });

        let line = serde_json::to_string(&event).unwrap() + "\n";

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.feedback_path) {
            file.write_all(line.as_bytes()).ok();
        }
    }

    /// Analyze feedback to detect user preferences and patterns.
    pub fn analyze_preferences(&self) -> Result<Value, String> {
        let content = fs::read_to_string(&self.feedback_path).unwrap_or_default();
        let mut corrections: HashMap<String, usize> = HashMap::new();
        let mut approvals: HashMap<String, usize> = HashMap::new();

        for line in content.lines() {
            if let Ok(event) = serde_json::from_str::<Value>(line) {
                let task_id = event["task_id"].as_str().unwrap_or("");
                match event["type"].as_str() {
                    Some("correction") => {
                        *corrections.entry(task_id.to_string()).or_insert(0) += 1;
                    }
                    Some("approval") => {
                        *approvals.entry(task_id.to_string()).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }

        // Identify tasks that frequently get corrected (need policy updates)
        let frequent_corrections: Vec<String> = corrections.iter()
            .filter(|(_, count)| **count >= 3)
            .map(|(task, _)| task.clone())
            .collect();

        Ok(json!({
            "type": "preference_analysis",
            "frequently_corrected_tasks": frequent_corrections,
            "recommendation": if !frequent_corrections.is_empty() {
                json!({
                    "action": "update_guard_policy",
                    "tasks": frequent_corrections,
                    "description": "Require approval before executing these task types"
                })
            } else {
                json!(null)
            }
        }))
    }

    /// Get correction history for a specific task type.
    pub fn get_corrections_for_task(&self, task_type: &str) -> Result<Vec<Value>, String> {
        let content = fs::read_to_string(&self.feedback_path).unwrap_or_default();
        let mut corrections = Vec::new();

        for line in content.lines() {
            if let Ok(event) = serde_json::from_str::<Value>(line) {
                if event["type"].as_str() == Some("correction") &&
                   event["details"]["task_type"].as_str() == Some(task_type) {
                    corrections.push(event);
                }
            }
        }

        Ok(corrections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_collector_initializes() {
        let collector = FeedbackCollector::new();
        assert!(collector.feedback_path.parent().is_some());
    }
}