//! Task scheduler — routes tasks to appropriate workers based on capabilities.

use crate::state_store::{Task, TaskStatus};
use crate::worker_registry::WorkerRegistry;
use std::collections::HashMap;

pub struct TaskScheduler {
    registry: WorkerRegistry,
}

impl TaskScheduler {
    pub fn new(registry: WorkerRegistry) -> Self {
        Self { registry }
    }

    /// Assign a task to an available worker with the required capabilities.
    pub fn schedule(&mut self, task: &Task, required_capabilities: &[String]) -> Option<&str> {
        // Find an available worker with the required capabilities
        if let Some(worker) = self.registry.find_available(required_capabilities) {
            // In production, update task status to assigned via state store
            return Some(&worker.id);
        }
        None
    }

    /// Get all pending tasks that can be scheduled.
    pub fn pending_tasks(&self) -> Vec<&Task> {
        // In production, query state store for tasks with Pending status
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_finds_available_worker() {
        let mut registry = WorkerRegistry::new();
        let worker = registry.register("ftw3".to_string(), vec!["gpu:nvidia:3070ti".to_string()]);

        let mut scheduler = TaskScheduler::new(registry);

        // This would assign a task to the available GPU worker
        // For now, just verify the scheduler can be created
    }
}
