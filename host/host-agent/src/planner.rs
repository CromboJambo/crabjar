/// Planner — generates task plans from understanding phase.
///
/// Takes the WorkItem's observations and produces a structured task list.
use crabjar_host_core::WorkItem;

pub struct Planner;

impl Planner {
    pub fn new() -> Self {
        Self
    }

    /// Generate a plan for the given WorkItem.
    ///
    /// Returns the number of tasks added.
    pub fn plan(&self, work_item: &mut WorkItem) -> usize {
        // Default: create a simple sequential plan based on the objective
        let objective = &work_item.objective;

        // Parse objective into sub-tasks (simple heuristic)
        let tasks = Self::parse_objective(objective);
        for task in tasks {
            work_item.add_task(task);
        }

        work_item.plan.len()
    }

    /// Parse a natural language objective into sub-tasks.
    fn parse_objective(objective: &str) -> Vec<String> {
        // For now, split on common delimiters or create a default task
        let parts: Vec<_> = objective
            .split(&[';', ',', '.'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.len() > 1 {
            parts.iter().map(|s| s.to_string()).collect()
        } else {
            vec![
                format!("Analyze: {}", objective),
                format!("Implement: {}", objective),
                format!("Verify: {}", objective),
            ]
        }
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}
