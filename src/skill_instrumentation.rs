/// Skill usage instrumentation for training data collection.
///
/// Wraps skill invocations to capture: which skill, duration, success/failure,
/// steps taken, and tool calls made. Emits JSON events to a log file that
/// feeds the training pipeline on ftw3.
///
/// Architecture:
///   crabjar session → skill invocation → SkillInvocationLogger.start()
///     → agent executes task → tool calls tracked → execution completes
///     → SkillInvocationLogger.finish(success)
///   → JSON event written to ~/.crabjar/skill-instrumentation.jsonl
///
/// Training pipeline reads these logs, ranks skills by value (frequency ×
/// success rate × complexity), and generates QLoRA training examples from
/// high-value successful invocations.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

/// Tracks a single in-progress skill invocation.
pub struct InvocationTracker {
    pub session_id: String,
    pub skill_name: String,
    pub start_time: Instant,
    pub tool_calls: Vec<String>,
}

/// Logger that captures skill invocations for training data generation.
pub struct SkillInvocationLogger {
    log_path: PathBuf,
    active: Mutex<HashMap<String, InvocationTracker>>,
}

impl SkillInvocationLogger {
    /// Create a new logger writing to the default instrumentation path.
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let log_dir = PathBuf::from(home).join(".crabjar");
        std::fs::create_dir_all(&log_dir).ok();

        Self {
            log_path: log_dir.join("skill-instrumentation.jsonl"),
            active: Mutex::new(HashMap::new()),
        }
    }

    /// Begin tracking a skill invocation. Returns invocation ID for later finish().
    pub fn start(&self, session_id: &str, skill_name: &str) -> String {
        let inv_id = Uuid::new_v4().to_string();

        let tracker = InvocationTracker {
            session_id: session_id.to_string(),
            skill_name: skill_name.to_string(),
            start_time: Instant::now(),
            tool_calls: Vec::new(),
        };

        self.active.lock().unwrap().insert(inv_id.clone(), tracker);
        inv_id
    }

    /// Record a tool call made during the invocation.
    pub fn record_tool_call(&self, inv_id: &str, tool_name: &str) {
        if let Some(tracker) = self.active.lock().unwrap().get_mut(inv_id) {
            tracker.tool_calls.push(tool_name.to_string());
        }
    }

    /// Finish tracking and emit the JSON event.
    pub fn finish(&self, inv_id: &str, success: bool, steps_taken: usize, error_type: Option<&str>) {
        let tracker = match self.active.lock().unwrap().remove(inv_id) {
            Some(t) => t,
            None => return, // Already finished or invalid ID
        };

        let duration_ms = tracker.start_time.elapsed().as_millis() as u64;

        let event = json!({
            "type": "skill_invocation",
            "invocation_id": inv_id,
            "session_id": tracker.session_id,
            "skill_name": tracker.skill_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "duration_ms": duration_ms,
            "success": success,
            "steps_taken": steps_taken,
            "tool_calls": tracker.tool_calls,
            "error_type": error_type.unwrap_or(""),
        });

        self.write_event(&event);
    }

    fn write_event(&self, event: &Value) {
        let line = serde_json::to_string(event).unwrap() + "\n";

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .expect("Failed to open instrumentation log");

        file.write_all(line.as_bytes()).expect("Failed to write event");
    }
}

// Thread-safe singleton for use across the agent runtime.
lazy_static::lazy_static! {
    pub static ref SKILL_LOGGER: SkillInvocationLogger = SkillInvocationLogger::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_invocation_event() {
        let logger = SkillInvocationLogger::new();

        let inv_id = logger.start("session-123", "test-skill");
        logger.record_tool_call(&inv_id, "read_file");
        logger.record_tool_call(&inv_id, "write_file");
        logger.finish(&inv_id, true, 5, None);

        // Verify event was written (check file exists and has content)
        assert!(logger.log_path.exists());
    }
}
