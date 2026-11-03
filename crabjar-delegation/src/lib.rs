//! Subagent delegation framework for crabjar.
//!
//! Provides `delegate_task` capability matching Hermes Agent's tool schema:
//! spawn isolated subagents in parallel, each with their own conversation
//! context and terminal session, then aggregate results when complete.
//!
//! # Architecture
//!
//! ```hemlock
//! Parent agent → delegate_task(tasks) → Dispatcher spawns N subprocesses
//!                                    ↓
//!                              Each child: isolated context + terminal
//!                                    ↓
//!                             Results aggregated and returned to parent
//! ```
//!
//! # Key Types
//!
//! - [`DelegationRequest`] — input to the delegation framework
//! - [`DelegatedTask`] — a single subagent task with goal and context
//! - [`DelegationResult`] — aggregated results from all children
//! - [`TaskDispatcher`] — manages spawning, lifecycle, and result collection

pub mod dispatcher;
pub mod aggregation;
pub mod timeout;
pub mod error;

pub use crate::dispatcher::TaskDispatcher as Dispatcher;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Action types for the `delegate_task` tool (Hermes-compatible).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DelegationAction {
    Spawn,
    List,
    Steer,
    Stop,
}

/// Input structure for the `delegate_task` tool (Hermes-compatible schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationInput {
    /// Tasks to spawn (required for spawn action).
    #[serde(default)]
    pub tasks: Vec<DelegatedTask>,
    /// Action to perform. Defaults to "spawn".
    #[serde(default = "default_action")]
    pub action: DelegationAction,
    /// Target task ID for steer/stop actions.
    #[serde(default)]
    pub subagent_id: Option<String>,
    /// Course-correction message for steer action.
    #[serde(default)]
    pub message: Option<String>,
}

fn default_action() -> DelegationAction {
    DelegationAction::Spawn
}

/// A single delegated subagent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// What this subagent should accomplish.
    pub goal: String,
    /// Background information the subagent needs.
    #[serde(default)]
    pub context: Option<String>,
    /// Optional model override for this specific task.
    #[serde(default)]
    pub model: Option<String>,
}

/// Request to delegate work to subagents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    /// Single task mode: goal and context.
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    /// Batch mode: array of tasks to run in parallel.
    #[serde(default)]
    pub tasks: Option<Vec<DelegatedTask>>,
    /// Return immediately without waiting for results.
    #[serde(default)]
    pub background: bool,
    /// Override model for all subagents.
    #[serde(default)]
    pub model: Option<String>,
}

/// Status of an individual delegated task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// Result from a single delegated subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Unique identifier for this task.
    pub task_id: String,
    /// The goal the subagent was asked to accomplish.
    pub goal: String,
    /// Final status of the task.
    pub status: TaskStatus,
    /// Subagent's final summary or result text.
    #[serde(default)]
    pub summary: Option<String>,
    /// Error message if the task failed.
    #[serde(default)]
    pub error: Option<String>,
    /// When this task was created.
    #[serde(default = "now_utc")]
    pub created_at: DateTime<Utc>,
}

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Aggregated results from a delegation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    /// Whether all tasks completed successfully.
    pub success: bool,
    /// Results from each delegated task.
    pub results: Vec<TaskResult>,
    /// Task IDs for background tracking.
    pub task_ids: Vec<String>,
}

impl DelegationRequest {
    /// Convert to a list of individual tasks (handles single vs batch mode).
    #[must_use]
    pub fn into_tasks(self) -> Vec<DelegatedTask> {
        if let Some(tasks) = self.tasks {
            return tasks;
        }

        if let Some(goal) = self.goal {
            vec![DelegatedTask {
                goal,
                context: self.context,
                model: self.model,
            }]
        } else {
            Vec::new()
        }
    }
}

impl DelegationResult {
    /// Create a success result.
    #[must_use]
    pub fn success(results: Vec<TaskResult>) -> Self {
        let task_ids = results.iter().map(|r| r.task_id.clone()).collect();
        Self {
            success: true,
            results,
            task_ids,
        }
    }

    /// Create a failure result.
    #[must_use]
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            results: vec![TaskResult {
                task_id: Uuid::new_v4().to_string(),
                goal: "delegation".to_string(),
                status: TaskStatus::Failed,
                summary: None,
                error: Some(error),
                created_at: now_utc(),
            }],
            task_ids: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_task_mode_converts_to_tasks() {
        let req = DelegationRequest {
            goal: Some("Research X".to_string()),
            context: Some("Background info".to_string()),
            tasks: None,
            background: false,
            model: None,
        };

        let tasks = req.into_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].goal, "Research X");
        assert_eq!(tasks[0].context, Some("Background info".to_string()));
    }

    #[test]
    fn batch_mode_preserves_tasks() {
        let req = DelegationRequest {
            goal: None,
            context: None,
            tasks: Some(vec![
                DelegatedTask { goal: "Task 1".to_string(), context: None, model: None },
                DelegatedTask { goal: "Task 2".to_string(), context: None, model: None },
            ]),
            background: false,
            model: None,
        };

        let tasks = req.into_tasks();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn delegation_result_success_format() {
        let result = DelegationResult::success(vec![TaskResult {
            task_id: "task-1".to_string(),
            goal: "Test".to_string(),
            status: TaskStatus::Completed,
            summary: Some("Done".to_string()),
            error: None,
            created_at: now_utc(),
        }]);

        assert!(result.success);
        assert_eq!(result.task_ids.len(), 1);
    }
}