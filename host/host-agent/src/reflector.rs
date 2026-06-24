/// Reflector — evaluates loop results and decides next steps.
///
/// After each iteration, the reflector answers:
/// - What succeeded?
/// - What failed?
/// - Confidence?
/// - Should I retry?
/// - Should I ask the user?
use crabjar_host_core::{Status, WorkItem};

pub struct Reflector;

impl Reflector {
    pub fn new() -> Self {
        Self
    }

    /// Reflect on the current WorkItem state.
    pub fn reflect(&self, work_item: &mut WorkItem) -> Reflection {
        let total = work_item.plan.len();
        let completed = work_item
            .plan
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crabjar_host_core::work_item::TaskStatus::Completed
                )
            })
            .count();
        let failed = work_item
            .plan
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crabjar_host_core::work_item::TaskStatus::Failed { .. }
                )
            })
            .count();

        let success_rate = if total > 0 {
            completed as f32 / total as f32
        } else {
            0.0
        };

        let should_retry = failed > 0 && work_item.plan.len() < 50;
        let should_ask_user = work_item.confidence < 0.3 && failed > 0;

        let next_status = if work_item.confidence >= 0.85 {
            Status::Completed
        } else if failed > completed && !should_retry {
            Status::Failed {
                reason: format!("Too many failures: {failed} failed, {completed} completed"),
            }
        } else {
            Status::Pending // Continue to next iteration
        };

        work_item.set_status(next_status.clone());

        Reflection {
            success_rate,
            completed,
            failed,
            total,
            confidence: work_item.confidence,
            should_retry,
            should_ask_user,
            next_status,
            summary: format!(
                "Progress: {completed}/{total} tasks completed, confidence: {:.0}%",
                work_item.confidence * 100.0
            ),
        }
    }
}

/// Reflection result.
#[derive(Debug, Clone)]
pub struct Reflection {
    pub success_rate: f32,
    pub completed: usize,
    pub failed: usize,
    pub total: usize,
    pub confidence: f32,
    pub should_retry: bool,
    pub should_ask_user: bool,
    pub next_status: Status,
    pub summary: String,
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new()
    }
}
