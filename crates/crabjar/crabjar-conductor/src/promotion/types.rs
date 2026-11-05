//! Promotion system types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Worker rank in the hierarchy.
/// - Rank 0: Task Worker — executes specific tasks
/// - Rank 1: Supervisor — oversees multiple workers, makes routing decisions
/// - Rank 2+: Manager — strategic planning, cross-service coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRank(pub u8);

impl WorkerRank {
    pub const TASK_WORKER: Self = Self(0);
    pub const SUPERVISOR: Self = Self(1);
    pub const MANAGER: Self = Self(2);

    #[must_use]
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Competence evidence collected from worker performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCompetenceEvidence {
    pub evidence_id: String,
    pub worker_id: String,
    pub timestamp: DateTime<Utc>,
    pub task_completions: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub decisions_made: u32,
    pub accurate_decisions: u32,
    pub knowledge_contributions: u32,
}

/// Promotion threshold configuration for a target rank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionThresholdConfig {
    pub target_rank: u8,
    pub minimum_tasks: u32,
    pub minimum_success_rate: f64,
    pub minimum_decisions: u32,
    pub minimum_accuracy_rate: f64,
    pub evidence_window_hours: u32,
}

/// Record of a completed promotion with full provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRecord {
    pub promotion_id: String,
    pub worker_id: String,
    pub from_rank: u8,
    pub to_rank: u8,
    pub timestamp: DateTime<Utc>,
    pub evidence_id: String,
    pub rationale: String,
    pub evaluator_id: Option<String>,
}

/// Result of a promotion eligibility evaluation.
#[derive(Debug, Serialize, Deserialize)]
pub struct PromotionEvaluation {
    pub eligible: bool,
    pub current_rank: u8,
    pub target_rank: u8,
    pub evidence_summary: WorkerCompetenceEvidence,
    pub thresholds_met: Vec<ThresholdCheck>,
    pub evidence_id: String,
}

/// Individual threshold check result.
#[derive(Debug, Serialize, Deserialize)]
pub struct ThresholdCheck {
    pub criterion: String,
    pub required: f64,
    pub actual: f64,
    pub met: bool,
}
