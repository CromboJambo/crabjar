//! Core orchestration logic — goal decomposition and task lifecycle management.

use crate::state_store::{Goal, GoalStatus, StateStore, Task, Worker};
use anyhow::Result;

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
    /// Create a new conductor with an in-memory database (for testing).
    pub fn new(config: ConductorConfig) -> Result<Self> {
        let store = StateStore::open(":memory:")?;
        Ok(Self { store, config })
    }

    /// Create a conductor with a specific database path.
    pub fn with_db(config: ConductorConfig, db_path: &str) -> Result<Self> {
        let store = StateStore::open(db_path)?;
        Ok(Self { store, config })
    }

    /// Submit a new goal for decomposition and execution.
    pub fn submit_goal(&self, description: String) -> Result<Goal> {
        // Create the goal record
        let goal = self.store.create_goal(description.clone())?;

        // Decompose into tasks using pesti-server (LLM inference)
        let _tasks = self.decompose(goal.id.clone(), &description)?;

        // Mark as running once decomposition is complete
        self.store.update_goal_status(&goal.id, GoalStatus::Running)?;

        Ok(goal)
    }

    /// Decompose a goal into executable tasks using LLM inference.
    fn decompose(&self, goal_id: String, description: &str) -> Result<Vec<Task>> {
        // In production, this calls pesti-server to break the goal into steps.
        // For now, create a placeholder task to demonstrate the flow.
        let task = self.store.create_task(&goal_id, format!("Execute: {}", description))?;

        Ok(vec![task])
    }

    /// Get status of a goal and its tasks.
    pub fn goal_status(&self, goal_id: &str) -> Result<Option<GoalStatus>> {
        let goal = self.store.get_goal(goal_id)?;
        Ok(goal.map(|g| g.status))
    }

    /// Get full goal details.
    pub fn get_goal(&self, goal_id: &str) -> Result<Option<Goal>> {
        self.store.get_goal(goal_id).map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List all registered workers.
    pub fn list_workers(&self) -> Vec<Worker> {
        // Workers are managed separately; for now return empty list
        vec![]
    }

    /// Poll for tasks assigned to a specific worker.
    pub fn tasks_for_worker(&self, _worker_id: &str) -> Result<Vec<Task>> {
        // In production, iterate over all tasks and filter by worker_id
        // This is a simplified version for the initial implementation
        Ok(vec![])
    }

    /// Report completion of a task.
    pub fn report_task_complete(&self, task_id: &str, result: Option<String>) -> Result<()> {
        self.store.complete_task(task_id, result)?;
        Ok(())
    }

    /// Get the state store (for testing and API layer).
    pub fn store(&self) -> &StateStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn submit_goal_creates_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = StateStore::open(&path).unwrap();
        let conductor = Conductor {
            store,
            config: ConductorConfig::default(),
        };

        let goal = conductor.submit_goal("Test goal".to_string()).unwrap();
        let status = conductor.goal_status(&goal.id).unwrap().unwrap();
        assert_eq!(status, GoalStatus::Running);
    }
}