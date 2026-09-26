//! Worker pool for parallel job execution.

use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Worker {
    id: u32,
    busy: bool,
}

impl Worker {
    fn new(id: u32) -> Self {
        Self { id, busy: false }
    }
}

pub struct WorkerPool {
    workers: Vec<Worker>,
}

impl WorkerPool {
    pub fn new(size: usize) -> Self {
        let workers = (0..size).map(|i| Worker::new(i as u32)).collect();
        Self { workers }
    }

    /// Spawn a job on an available worker. Returns None if all workers are busy.
    pub fn spawn<F>(&mut self, f: F) -> Option<JoinHandle<String>>
    where
        F: FnOnce() -> String + Send + 'static,
    {
        let worker = self.workers.iter_mut().find(|w| !w.busy)?;
        worker.busy = true;
        Some(tokio::task::spawn_blocking(f))
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn busy_workers(&self) -> usize {
        self.workers.iter().filter(|w| w.busy).count()
    }

    /// Mark a worker as available again.
    pub fn mark_available(&mut self, worker_id: u32) {
        if let Some(worker) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            worker.busy = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = WorkerPool::new(4);
        assert_eq!(pool.worker_count(), 4);
        assert_eq!(pool.busy_workers(), 0);
    }
}
