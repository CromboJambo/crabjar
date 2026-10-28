//! Evidence-based worker promotion system.
//!
//! Collects competence metrics from worker execution outcomes and evaluates
//! eligibility for rank elevation based on configurable thresholds.
//!
//! Architecture:
//! ```hemlock
//! Worker Performance → Evidence Collector → Competence Evaluator → Promotion Engine
//!      ↑                    ↓                     ↓                      ↓
//! Task Execution      SQLite storage       Threshold checks        Rank update + audit log
//! ```

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Promotion manager that coordinates evidence collection and evaluation.
pub struct PromotionManager {
    db: Connection,
}

impl PromotionManager {
    /// Create a new promotion manager with the given database connection.
    pub fn new(db: Connection) -> Result<Self, rusqlite::Error> {
        let manager = Self { db };
        manager.init_schema()?;
        Ok(manager)
    }

    /// Initialize SQLite schema for promotion tracking.
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS worker_competence_evidence (
                evidence_id TEXT PRIMARY KEY,
                worker_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                task_completions INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                failure_count INTEGER NOT NULL DEFAULT 0,
                decisions_made INTEGER NOT NULL DEFAULT 0,
                accurate_decisions INTEGER NOT NULL DEFAULT 0,
                knowledge_contributions INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS promotion_records (
                promotion_id TEXT PRIMARY KEY,
                worker_id TEXT NOT NULL,
                from_rank INTEGER NOT NULL,
                to_rank INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                evidence_id TEXT NOT NULL REFERENCES worker_competence_evidence(evidence_id),
                rationale TEXT NOT NULL,
                evaluator_id TEXT
            );

            CREATE TABLE IF NOT EXISTS promotion_thresholds (
                target_rank INTEGER PRIMARY KEY,
                minimum_tasks INTEGER NOT NULL DEFAULT 50,
                minimum_success_rate REAL NOT NULL DEFAULT 0.85,
                minimum_decisions INTEGER NOT NULL DEFAULT 10,
                minimum_accuracy_rate REAL NOT NULL DEFAULT 0.80,
                evidence_window_hours INTEGER NOT NULL DEFAULT 720
            );

            CREATE TABLE IF NOT EXISTS worker_ranks (
                worker_id TEXT PRIMARY KEY,
                rank INTEGER NOT NULL DEFAULT 0
            );

            INSERT OR IGNORE INTO promotion_thresholds (target_rank, minimum_tasks, minimum_success_rate)
            VALUES (1, 50, 0.85);

            INSERT OR IGNORE INTO promotion_thresholds (target_rank, minimum_tasks, minimum_success_rate, minimum_decisions, minimum_accuracy_rate)
            VALUES (2, 200, 0.90, 50, 0.85);",
        )?;
        Ok(())
    }

    /// Get or create rank entry for a worker.
    pub fn get_worker_rank(&self, worker_id: &str) -> Result<u8, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT rank FROM worker_ranks WHERE worker_id = ?",
            params![worker_id],
            |r| r.get::<_, u8>(0),
        );

        match row {
            Ok(rank) => Ok(rank),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.db.execute(
                    "INSERT INTO worker_ranks (worker_id, rank) VALUES (?, 0)",
                    params![worker_id],
                )?;
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    /// Update a worker's rank.
    pub fn set_worker_rank(&self, worker_id: &str, rank: u8) -> Result<(), rusqlite::Error> {
        self.db.execute(
            "INSERT OR REPLACE INTO worker_ranks (worker_id, rank) VALUES (?, ?)",
            params![worker_id, rank],
        )?;
        Ok(())
    }

    /// Record a task completion as evidence.
    pub fn record_task_completion(
        &self,
        worker_id: &str,
        success: bool,
    ) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let evidence_id = format!("ev_{}", Uuid::new_v4());

        // Create or update today's evidence record for this worker
        self.db.execute(
            "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp, task_completions, success_count, failure_count)
             VALUES (?, ?, ?, 1, ?, ?)",
            params![evidence_id, worker_id, now, if success { 1 } else { 0 }, if success { 0 } else { 1 }],
        )?;

        Ok(())
    }

    /// Record a decision outcome as evidence.
    pub fn record_decision(
        &self,
        worker_id: &str,
        accurate: bool,
    ) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let evidence_id = format!("ev_{}", Uuid::new_v4());

        self.db.execute(
            "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp, decisions_made, accurate_decisions)
             VALUES (?, ?, ?, 1, ?)",
            params![evidence_id, worker_id, now, if accurate { 1 } else { 0 }],
        )?;

        Ok(())
    }

    /// Aggregate evidence for a worker within a time window.
    fn aggregate_evidence(&self, worker_id: &str, window_hours: u32) -> Result<WorkerCompetenceEvidence, rusqlite::Error> {
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::hours(window_hours as i64))
            .unwrap_or(Utc::now())
            .to_rfc3339();

        let row = self.db.query_row(
            "SELECT COALESCE(SUM(task_completions), 0), COALESCE(SUM(success_count), 0),
                    COALESCE(SUM(failure_count), 0), COALESCE(SUM(decisions_made), 0),
                    COALESCE(SUM(accurate_decisions), 0), COALESCE(SUM(knowledge_contributions), 0)
             FROM worker_competence_evidence
             WHERE worker_id = ? AND timestamp >= ?",
            params![worker_id, cutoff],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            )),
        )?;

        let (completions, successes, failures, decisions, accurate, contributions) = row;

        Ok(WorkerCompetenceEvidence {
            evidence_id: format!("agg_{}", Uuid::new_v4()),
            worker_id: worker_id.to_string(),
            timestamp: Utc::now(),
            task_completions: completions as u32,
            success_count: successes as u32,
            failure_count: failures as u32,
            decisions_made: decisions as u32,
            accurate_decisions: accurate as u32,
            knowledge_contributions: contributions as u32,
        })
    }

    /// Get threshold configuration for a target rank.
    fn get_thresholds(&self, target_rank: u8) -> Result<PromotionThresholdConfig, rusqlite::Error> {
        let row = self.db.query_row(
            "SELECT minimum_tasks, minimum_success_rate, minimum_decisions, minimum_accuracy_rate, evidence_window_hours
             FROM promotion_thresholds WHERE target_rank = ?",
            params![target_rank],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, f64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, i64>(4)?,
            )),
        );

        match row {
            Ok((tasks, success_rate, decisions, accuracy_rate, window)) => Ok(PromotionThresholdConfig {
                target_rank,
                minimum_tasks: tasks as u32,
                minimum_success_rate: success_rate,
                minimum_decisions: decisions as u32,
                minimum_accuracy_rate: accuracy_rate,
                evidence_window_hours: window as u32,
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(PromotionThresholdConfig {
                target_rank,
                minimum_tasks: 50,
                minimum_success_rate: 0.85,
                minimum_decisions: 10,
                minimum_accuracy_rate: 0.80,
                evidence_window_hours: 720,
            }),
            Err(e) => Err(e),
        }
    }

    /// Evaluate whether a worker is eligible for promotion to a target rank.
    pub fn evaluate_promotion(
        &self,
        worker_id: &str,
        target_rank: u8,
    ) -> Result<PromotionEvaluation, rusqlite::Error> {
        let current_rank = self.get_worker_rank(worker_id)?;
        let thresholds = self.get_thresholds(target_rank)?;
        let evidence = self.aggregate_evidence(worker_id, thresholds.evidence_window_hours)?;

        // Evaluate each threshold criterion
        let mut checks = Vec::new();

        // Task completion volume
        let tasks_met = evidence.task_completions >= thresholds.minimum_tasks;
        checks.push(ThresholdCheck {
            criterion: "minimum_tasks".to_string(),
            required: thresholds.minimum_tasks as f64,
            actual: evidence.task_completions as f64,
            met: tasks_met,
        });

        // Success rate
        let success_rate = if evidence.task_completions > 0 {
            evidence.success_count as f64 / evidence.task_completions as f64
        } else {
            0.0
        };
        let success_met = success_rate >= thresholds.minimum_success_rate;
        checks.push(ThresholdCheck {
            criterion: "success_rate".to_string(),
            required: thresholds.minimum_success_rate,
            actual: success_rate,
            met: success_met,
        });

        // Decision volume (for supervisor+ ranks)
        if target_rank >= 1 {
            let decisions_met = evidence.decisions_made >= thresholds.minimum_decisions;
            checks.push(ThresholdCheck {
                criterion: "minimum_decisions".to_string(),
                required: thresholds.minimum_decisions as f64,
                actual: evidence.decisions_made as f64,
                met: decisions_met,
            });

            // Decision accuracy
            let accuracy_rate = if evidence.decisions_made > 0 {
                evidence.accurate_decisions as f64 / evidence.decisions_made as f64
            } else {
                0.0
            };
            let accuracy_met = accuracy_rate >= thresholds.minimum_accuracy_rate;
            checks.push(ThresholdCheck {
                criterion: "accuracy_rate".to_string(),
                required: thresholds.minimum_accuracy_rate,
                actual: accuracy_rate,
                met: accuracy_met,
            });
        }

        let eligible = checks.iter().all(|c| c.met) && current_rank < target_rank;

        Ok(PromotionEvaluation {
            eligible,
            current_rank,
            target_rank,
            evidence_summary: evidence,
            thresholds_met: checks,
            evidence_id: format!("eval_{}", Uuid::new_v4()),
        })
    }

    /// Execute a promotion for an eligible worker.
    pub fn execute_promotion(
        &self,
        worker_id: &str,
        target_rank: u8,
        rationale: &str,
    ) -> Result<PromotionRecord, rusqlite::Error> {
        let current_rank = self.get_worker_rank(worker_id)?;

        if current_rank >= target_rank {
            return Err(rusqlite::Error::InvalidParameterName("Already at or above target rank".to_string()));
        }

        // Evaluate eligibility first
        let evaluation = self.evaluate_promotion(worker_id, target_rank)?;
        if !evaluation.eligible {
            return Err(rusqlite::Error::InvalidParameterName("Not eligible for promotion".to_string()));
        }

        // Create an actual evidence record for this promotion decision
        let now = Utc::now().to_rfc3339();
        let evidence_id = format!("pr_ev_{}", Uuid::new_v4());
        self.db.execute(
            "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp) VALUES (?, ?, ?)",
            params![evidence_id, worker_id, now],
        )?;

        let promotion_id = format!("pr_{}", Uuid::new_v4());

        // Record the promotion
        self.db.execute(
            "INSERT INTO promotion_records (promotion_id, worker_id, from_rank, to_rank, timestamp, evidence_id, rationale)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                promotion_id,
                worker_id,
                current_rank,
                target_rank,
                now,
                evidence_id,
                rationale
            ],
        )?;

        // Update worker rank
        self.set_worker_rank(worker_id, target_rank)?;

        Ok(PromotionRecord {
            promotion_id,
            worker_id: worker_id.to_string(),
            from_rank: current_rank,
            to_rank: target_rank,
            timestamp: Utc::now(),
            evidence_id,
            rationale: rationale.to_string(),
            evaluator_id: None,
        })
    }

    /// Get promotion history for a worker.
    pub fn get_promotion_history(&self, worker_id: &str) -> Result<Vec<PromotionRecord>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT promotion_id, worker_id, from_rank, to_rank, timestamp, evidence_id, rationale, evaluator_id
             FROM promotion_records WHERE worker_id = ? ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map(params![worker_id], |r| {
            Ok(PromotionRecord {
                promotion_id: r.get(0)?,
                worker_id: r.get(1)?,
                from_rank: r.get(2)?,
                to_rank: r.get(3)?,
                timestamp: DateTime::parse_from_rfc3339(&r.get::<_, String>(4)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                evidence_id: r.get(5)?,
                rationale: r.get(6)?,
                evaluator_id: r.get(7).ok(),
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Get raw competence metrics for a worker.
    pub fn get_worker_metrics(&self, worker_id: &str) -> Result<WorkerCompetenceEvidence, rusqlite::Error> {
        self.aggregate_evidence(worker_id, 720) // Last 30 days by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promotion_schema_creation() {
        let db = Connection::open_in_memory().unwrap();
        let manager = PromotionManager::new(db).unwrap();
        
        // Verify tables were created by querying the manager's database
        // (db was moved into manager, so we can't query it directly)
        // Instead, verify by checking that get_worker_rank works (requires schema)
        let rank = manager.get_worker_rank("test_worker").unwrap();
        assert_eq!(rank, 0);
    }

    #[test]
    fn test_worker_rank_management() {
        let db = Connection::open_in_memory().unwrap();
        let manager = PromotionManager::new(db).unwrap();

        // New worker defaults to rank 0
        assert_eq!(manager.get_worker_rank("w1").unwrap(), 0);

        // Can set rank
        manager.set_worker_rank("w1", 1).unwrap();
        assert_eq!(manager.get_worker_rank("w1").unwrap(), 1);
    }

    #[test]
    fn test_evidence_collection_and_evaluation() {
        let db = Connection::open_in_memory().unwrap();
        let manager = PromotionManager::new(db).unwrap();

        // Record task completions (need 50 for rank 1)
        for i in 0..48 {
            manager.record_task_completion("w1", true).unwrap();
        }
        
        // Not yet eligible (only 48 tasks)
        let eval = manager.evaluate_promotion("w1", 1).unwrap();
        assert!(!eval.eligible);

        // Add 2 more successful tasks
        manager.record_task_completion("w1", true).unwrap();
        manager.record_task_completion("w1", true).unwrap();

        // Record some decisions (need 10 for supervisor rank)
        for i in 0..8 {
            manager.record_decision("w1", true).unwrap();
        }
        
        // Still not eligible (only 8 decisions)
        let eval = manager.evaluate_promotion("w1", 1).unwrap();
        assert!(!eval.eligible);

        // Add 2 more accurate decisions
        manager.record_decision("w1", true).unwrap();
        manager.record_decision("w1", true).unwrap();

        // Now eligible (50 tasks, 100% success rate, 10 decisions, 100% accuracy)
        let eval = manager.evaluate_promotion("w1", 1).unwrap();
        assert!(eval.eligible);
        assert_eq!(eval.evidence_summary.task_completions, 50);
        assert_eq!(eval.evidence_summary.decisions_made, 10);
    }

    #[test]
    fn test_promotion_execution() {
        let db = Connection::open_in_memory().unwrap();
        let manager = PromotionManager::new(db).unwrap();

        // Build up enough evidence (50 tasks + 10 decisions)
        for _ in 0..60 {
            manager.record_task_completion("w1", true).unwrap();
        }
        for _ in 0..12 {
            manager.record_decision("w1", true).unwrap();
        }

        // Execute promotion
        let record = manager.execute_promotion("w1", 1, "Test promotion").unwrap();
        assert_eq!(record.from_rank, 0);
        assert_eq!(record.to_rank, 1);
        assert_eq!(manager.get_worker_rank("w1").unwrap(), 1);

        // Verify history
        let history = manager.get_promotion_history("w1").unwrap();
        assert_eq!(history.len(), 1);
    }
}
