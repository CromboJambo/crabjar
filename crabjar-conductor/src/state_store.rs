//! State store abstraction for conductor persistence.
//!
//! Uses embedded SQLite for simplicity; can migrate to Postgres later
//! if scale demands it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// High-level objective submitted by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Decomposed,
    Running,
    Completed,
    Failed,
}

/// Executable unit of work with dependency tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub goal_id: String,
    pub worker_id: Option<String>,
    pub command: String,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

/// Registered fleet member with capability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub ssh_key: Option<String>,
    pub capabilities: Vec<String>,
    pub status: WorkerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerStatus {
    Online,
    Busy,
    Offline,
}

/// In-memory state store (SQLite-backed in production).
pub struct StateStore {
    goals: HashMap<String, Goal>,
    tasks: HashMap<String, Task>,
    workers: HashMap<String, Worker>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            tasks: HashMap::new(),
            workers: HashMap::new(),
        }
    }

    // Goal operations
    pub fn create_goal(&mut self, description: String) -> Goal {
        let goal = Goal {
            id: Uuid::new_v4().to_string(),
            description,
            status: GoalStatus::Decomposed,
            created_at: Utc::now(),
        };
        self.goals.insert(goal.id.clone(), goal.clone());
        goal
    }

    pub fn get_goal(&self, id: &str) -> Option<&Goal> {
        self.goals.get(id)
    }

    pub fn update_goal_status(&mut self, id: &str, status: GoalStatus) {
        if let Some(goal) = self.goals.get_mut(id) {
            goal.status = status;
        }
    }

    // Task operations
    pub fn create_task(&mut self, goal_id: String, command: String) -> Task {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            goal_id,
            worker_id: None,
            command,
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
        };
        self.tasks.insert(task.id.clone(), task.clone());
        task
    }

    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn assign_task(&mut self, task_id: &str, worker_id: &str) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.worker_id = Some(worker_id.to_string());
            task.status = TaskStatus::Assigned;
        }
    }

    pub fn complete_task(&mut self, task_id: &str, result: Option<String>) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = TaskStatus::Completed;
            task.result = result;
        }
    }

    // Worker operations
    pub fn register_worker(&mut self, name: String, capabilities: Vec<String>) -> Worker {
        let worker = Worker {
            id: Uuid::new_v4().to_string(),
            name,
            ssh_key: None,
            capabilities,
            status: WorkerStatus::Online,
        };
        self.workers.insert(worker.id.clone(), worker.clone());
        worker
    }

    pub fn get_worker(&self, id: &str) -> Option<&Worker> {
        self.workers.get(id)
    }

    pub fn list_workers(&self) -> Vec<&Worker> {
        self.workers.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_goal() {
        let mut store = StateStore::new();
        let goal = store.create_goal("Test goal".to_string());
        assert_eq!(store.get_goal(&goal.id).unwrap().description, "Test goal");
    }

    #[test]
    fn worker_registration() {
        let mut store = StateStore::new();
        let worker = store.register_worker("ftw3".to_string(), vec!["gpu:nvidia:3070ti"]);
        assert_eq!(store.get_worker(&worker.id).unwrap().name, "ftw3");
    }
}
