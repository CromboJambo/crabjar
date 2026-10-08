//! Worker registry for tracking fleet members and their capabilities.

use crate::state_store::{Worker, WorkerStatus};
use std::collections::HashMap;

pub struct WorkerRegistry {
    workers: HashMap<String, Worker>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, capabilities: Vec<String>) -> Worker {
        let worker = Worker {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            ssh_key: None,
            capabilities,
            status: WorkerStatus::Online,
        };
        self.workers.insert(worker.id.clone(), worker.clone());
        worker
    }

    pub fn get(&self, id: &str) -> Option<&Worker> {
        self.workers.get(id)
    }

    pub fn list(&self) -> Vec<&Worker> {
        self.workers.values().collect()
    }

    /// Find workers with a specific capability.
    pub fn find_by_capability(&self, capability: &str) -> Vec<&Worker> {
        self.workers
            .values()
            .filter(|w| w.capabilities.contains(&capability.to_string()))
            .collect()
    }

    /// Find an online worker with the required capabilities.
    pub fn find_available(&self, required: &[String]) -> Option<&Worker> {
        self.workers.values().find(|w| {
            if w.status != WorkerStatus::Online {
                return false;
            }
            required.iter().all(|cap| w.capabilities.contains(cap))
        })
    }

    pub fn set_status(&mut self, id: &str, status: WorkerStatus) {
        if let Some(worker) = self.workers.get_mut(id) {
            worker.status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_find() {
        let mut reg = WorkerRegistry::new();
        let w = reg.register("ftw3".to_string(), vec!["gpu:nvidia:3070ti".to_string()]);
        assert_eq!(reg.get(&w.id).unwrap().name, "ftw3");
    }

    #[test]
    fn find_by_capability() {
        let mut reg = WorkerRegistry::new();
        reg.register("cpu-node".to_string(), vec!["cpu:8core".to_string()]);
        let gpu_worker = reg.register("gpu-node".to_string(), vec!["gpu:nvidia:3070ti".to_string()]);
        let found = reg.find_by_capability("gpu:nvidia:3070ti");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, gpu_worker.id);
    }
}
