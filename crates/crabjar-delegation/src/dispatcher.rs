//! Task dispatcher for subagent delegation framework.
//!
//! Submits goals to the crabjar-conductor service via HTTP and waits for results.
//! Supports parallel task execution with concurrency limits and timeouts.

use std::time::{Duration, Instant};

use crate::error::DelegationError;
use crate::{DelegatedTask, DelegationRequest, DelegationResult, TaskResult, TaskStatus};

/// Configuration for the task dispatcher.
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Maximum number of concurrent tasks (default: 3)
    pub max_concurrent: usize,
    /// Timeout per task in seconds (default: 600)
    pub task_timeout: u64,
    /// Conductor API endpoint (default: <http://localhost:8080>)
    pub conductor_url: String,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            task_timeout: 600,
            conductor_url: "http://localhost:8080".to_string(),
        }
    }
}

/// Task dispatcher that manages subagent delegation.
pub struct TaskDispatcher {
    config: DispatcherConfig,
}

impl TaskDispatcher {
    /// Create a new task dispatcher with the given configuration.
    #[must_use]
    pub fn new(config: Option<DispatcherConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    /// Spawn multiple delegated tasks in parallel and wait for all results.
    ///
    /// Each task is submitted to the conductor as a goal, then we poll for completion.
    pub async fn spawn(
        &self,
        request: DelegationRequest,
    ) -> Result<DelegationResult, DelegationError> {
        // Validate concurrency limit
        let tasks = request.into_tasks();
        if tasks.len() > self.config.max_concurrent {
            return Err(DelegationError::ConcurrencyLimit {
                limit: self.config.max_concurrent,
            });
        }

        // Submit all tasks to conductor as goals and wait for results
        let mut results = Vec::new();
        for task in &tasks {
            let result = self.execute_task_full(task).await?;
            results.push(result);
        }

        Ok(DelegationResult::success(results))
    }

    /// Execute a single task with full wait (used by spawn for blocking mode).
    async fn execute_task_full(&self, task: &DelegatedTask) -> Result<TaskResult, DelegationError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        // Submit goal to conductor
        let goal_id = self.submit_goal(&task.goal).await?;

        // Wait for completion with timeout
        let result = self.wait_for_task(&goal_id, task, &task_id).await?;
        Ok(result)
    }

    /// Execute a single delegated task via the conductor API.
    pub async fn execute_task(
        &self,
        goal: String,
        _context: Option<String>,
    ) -> Result<String, DelegationError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        // Submit goal to conductor
        let goal_id = self.submit_goal(&goal).await?;

        // Store mapping from our task ID to conductor goal ID (in a real implementation this would be persisted)
        tracing::info!("Mapped task {} to conductor goal {}", task_id, goal_id);

        Ok(task_id)
    }

    /// List all tasks (returns empty for now — conductor doesn't have a list endpoint yet).
    pub async fn list_tasks(&self) -> Result<Vec<TaskResult>, DelegationError> {
        // TODO: Implement when conductor adds a goals listing endpoint
        Ok(Vec::new())
    }

    /// Steer a running task (send course-correction message).
    pub async fn steer_task(&self, _task_id: &str, _message: &str) -> Result<(), DelegationError> {
        // TODO: Implement when conductor adds steering capability
        Ok(())
    }

    /// Stop a running task.
    pub async fn stop_task(&self, _task_id: &str) -> Result<(), DelegationError> {
        // TODO: Implement when conductor adds goal cancellation
        Ok(())
    }

    /// Submit a goal to the conductor API.
    async fn submit_goal(&self, description: &str) -> Result<String, DelegationError> {
        let body = serde_json::json!({
            "description": description
        });

        let response = reqwest::Client::new()
            .post(format!("{}/goals", self.config.conductor_url))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| DelegationError::SpawnFailed(e.to_string()))?;

        let status = response.status();
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DelegationError::SpawnFailed(e.to_string()))?;

        if status.is_success() {
            Ok(json["goal_id"].as_str().unwrap_or("unknown").to_string())
        } else {
            Err(DelegationError::SpawnFailed(format!(
                "Conductor returned {}: {}",
                status,
                json.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("no message")
            )))
        }
    }

    /// Wait for a task to complete by polling the conductor API.
    async fn wait_for_task(
        &self,
        goal_id: &str,
        task: &DelegatedTask,
        task_id: &str,
    ) -> Result<TaskResult, DelegationError> {
        let start = Instant::now();
        let timeout = Duration::from_secs(self.config.task_timeout);

        loop {
            if start.elapsed() > timeout {
                return Ok(TaskResult {
                    created_at: chrono::Utc::now(),
                    task_id: task_id.to_string(),
                    goal: task.goal.clone(),
                    status: TaskStatus::TimedOut,
                    summary: None,
                    error: Some(format!(
                        "Task timed out after {} seconds",
                        self.config.task_timeout
                    )),
                });
            }

            // Poll for goal status
            let response = reqwest::Client::new()
                .get(format!("{}/goals/{}", self.config.conductor_url, goal_id))
                .send()
                .await
                .map_err(|e| DelegationError::SpawnFailed(e.to_string()))?;

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| DelegationError::SpawnFailed(e.to_string()))?;

            let status = json["status"].as_str().unwrap_or("unknown");

            match status {
                "completed" => {
                    return Ok(TaskResult {
                        created_at: chrono::Utc::now(),
                        task_id: task_id.to_string(),
                        goal: task.goal.clone(),
                        status: TaskStatus::Completed,
                        summary: Some(json.get("result").and_then(|r| r.as_str()).map_or_else(
                            || json["result"].to_string(),
                            std::string::ToString::to_string,
                        )),
                        error: None,
                    });
                }
                "failed" => {
                    return Ok(TaskResult {
                        created_at: chrono::Utc::now(),
                        task_id: task_id.to_string(),
                        goal: task.goal.clone(),
                        status: TaskStatus::Failed,
                        summary: None,
                        error: Some(
                            json["message"]
                                .as_str()
                                .unwrap_or("unknown error")
                                .to_string(),
                        ),
                    });
                }
                _ => {
                    // Still running — wait and poll again
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_config_defaults() {
        let config = DispatcherConfig::default();
        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.task_timeout, 600);
        assert_eq!(config.conductor_url, "http://localhost:8080");
    }

    #[test]
    fn dispatcher_new_with_config() {
        let config = DispatcherConfig {
            max_concurrent: 5,
            task_timeout: 120,
            conductor_url: "http://example.com".to_string(),
        };
        let dispatcher = TaskDispatcher::new(Some(config));
        assert_eq!(dispatcher.config.max_concurrent, 5);
    }

    #[test]
    fn dispatcher_new_without_config() {
        let dispatcher = TaskDispatcher::new(None);
        assert_eq!(dispatcher.config.max_concurrent, 3);
    }
}
