//! Result aggregation — collect and merge outputs from parallel subagents.

use crate::{DelegationResult, TaskResult, TaskStatus};

/// Aggregator for collecting results from multiple delegated tasks.
pub struct ResultAggregator {
    results: Vec<TaskResult>,
}

impl ResultAggregator {
    /// Create a new result aggregator.
    #[must_use]
    pub fn new() -> Self {
        Self { results: Vec::new() }
    }

    /// Add a task result to the aggregation.
    pub fn add_result(&mut self, result: TaskResult) {
        self.results.push(result);
    }

    /// Check if all tasks have completed (successfully or failed).
    #[must_use]
    pub fn all_complete(&self) -> bool {
        self.results.iter().all(|r| matches!(
            r.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::TimedOut
        ))
    }

    /// Count of successfully completed tasks.
    #[must_use]
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.status == TaskStatus::Completed).count()
    }

    /// Count of failed tasks.
    #[must_use]
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| matches!(r.status, TaskStatus::Failed | TaskStatus::TimedOut)).count()
    }

    /// Finalize and return the aggregated delegation result.
    #[must_use]
    pub fn finalize(self) -> DelegationResult {
        let success = self.failure_count() == 0;
        DelegationResult {
            success,
            results: self.results,
            task_ids: Vec::new(), // Will be populated by caller
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregator_tracks_results() {
        let mut agg = ResultAggregator::new();
        agg.add_result(TaskResult {
            task_id: "1".to_string(),
            goal: "Test".to_string(),
            status: TaskStatus::Completed,
            summary: Some("Done".to_string()),
            error: None,
            created_at: crate::now_utc(),
        });
        assert_eq!(agg.success_count(), 1);
    }

    #[test]
    fn aggregator_detects_completion() {
        let mut agg = ResultAggregator::new();
        agg.add_result(TaskResult {
            task_id: "1".to_string(),
            goal: "Test".to_string(),
            status: TaskStatus::Completed,
            summary: None,
            error: None,
            created_at: crate::now_utc(),
        });
        assert!(agg.all_complete());
    }
}