/// TaskExecutor — runs individual tasks defined in a WorkItem's plan.
///
/// Each task is independently executable. Results are captured and stored.

use crabjar_host_core::{WorkItem, work_item::TaskStatus};

pub struct TaskExecutor;

impl TaskExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a single task by index.
    pub fn execute_task(&self, work_item: &mut WorkItem, task_id: usize) -> Result<String, String> {
        if task_id >= work_item.plan.len() {
            return Err(format!("task {} out of range (plan has {} tasks)", task_id, work_item.plan.len()));
        }

        let task_desc = work_item.plan[task_id].description.clone();
        work_item.update_task(task_id, TaskStatus::InProgress, None);
        work_item.observe("execute", "task", format!("Executing: {task_desc}"));

        // Default: simulate task execution (in practice, this would run commands,
        // invoke tools, call APIs, etc.)
        let result = format!("Task '{task_desc}' executed successfully");
        work_item.update_task(task_id, TaskStatus::Completed, Some(result.clone()));
        work_item.add_artifact(&result);

        Ok(result)
    }

    /// Execute all pending tasks in the WorkItem's plan.
    pub fn execute_all(&self, work_item: &mut WorkItem) -> Vec<(usize, Result<String, String>)> {
        let results: Vec<_> = (0..work_item.plan.len())
            .map(|id| (id, self.execute_task(work_item, id)))
            .collect();

        work_item.observe("execute", "batch", format!("Executed {} tasks", results.len()));
        results
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}
