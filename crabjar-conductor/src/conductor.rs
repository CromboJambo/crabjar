//! Core orchestration logic — goal decomposition and task lifecycle management.

use crate::state_store::{Goal, GoalStatus, StateStore, Task, TaskStatus};
use anyhow::{anyhow, Result};

/// Configuration for the conductor service.
pub struct ConductorConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub pesti_server_url: String,
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8091,
            pesti_server_url: "http://localhost:8000".to_string(),
        }
    }
}

/// The conductor — receives goals, decomposes them into tasks, manages lifecycle.
pub struct Conductor {
    store: StateStore,
    config: ConductorConfig,
}

impl Conductor {
    pub fn new(config: ConductorConfig) -> Self {
        Self {
            store: StateStore::new(),
            config,
        }
    }

    /// Submit a new goal for decomposition and execution.
    pub async fn submit_goal(&mut self, description: String) -> Result<Goal> {
        // Create the goal record
        let goal = self.store.create_goal(description.clone());

        // Decompose into tasks using pesti-server (LLM inference)
        let tasks = self.decompose(goal.id.clone(), &description).await?;

        // Mark as running once decomposition is complete
        self.store.update_goal_status(&goal.id, GoalStatus::Running);

        Ok(goal)
    }

    /// Decompose a goal into executable tasks using LLM inference.
    async fn decompose(&self, goal_id: String, description: &str) -> Result<Vec<Task>> {
        // In production, this calls pesti-server to break the goal into steps.
        // For now, create a placeholder task to demonstrate the flow.
        let task = self.store.create_task(goal_id.clone(), format!("Execute: {}", description));

        Ok(vec![task])
    }

    /// Get status of a goal and its tasks.
    pub fn goal_status(&self, goal_id: &str) -> Option<GoalStatus> {
        self.store.get_goal(goal_id).map(|g| g.status.clone())
    }

    /// Poll for tasks assigned to a specific worker.
    pub fn tasks_for_worker(&self, worker_id: &str) -> Vec<&Task> {
        let mut results = vec![];
        // In production, iterate over all tasks and filter by worker_id
        // This is a simplified version for the initial implementation
        results
    }

    /// Report completion of a task.
    pub fn report_task_complete(&mut self, task_id: &str, result: Option<String>) {
        self.store.complete_task(task_id, result);
    }

    /// Get the state store (for testing and API layer).
    pub fn store(&self) -> &StateStore {
        &self.store
    }

    /// Get mutable reference to the state store.
    pub fn store_mut(&mut self) -> &mut StateStore {
        &mut self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn submit_goal_creates_task() {
        let mut conductor = Conductor::new(ConductorConfig::default());
        let goal = conductor.submit_goal("Test goal".to_string()).await.unwrap();
        assert_eq!(conductor.goal_status(&goal.id), Some(GoalStatus::Running));
    }
}
