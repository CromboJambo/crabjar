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

pub(crate) mod evaluation;
pub(crate) mod evidence;
pub(crate) mod types;

// Re-export public types
pub use self::types::{
    PromotionEvaluation, PromotionRecord, PromotionThresholdConfig, ThresholdCheck,
    WorkerCompetenceEvidence, WorkerRank,
};

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
            "CREATE TABLE IF NOT EXISTS worker_competence_evidence (\n\
             evidence_id TEXT PRIMARY KEY,\n\
             worker_id TEXT NOT NULL,\n\
             timestamp TEXT NOT NULL,\n\
             task_completions INTEGER NOT NULL DEFAULT 0,\n\
             success_count INTEGER NOT NULL DEFAULT 0,\n\
             failure_count INTEGER NOT NULL DEFAULT 0,\n\
             decisions_made INTEGER NOT NULL DEFAULT 0,\n\
             accurate_decisions INTEGER NOT NULL DEFAULT 0,\n\
             knowledge_contributions INTEGER NOT NULL DEFAULT 0\n\
             );\n\
             \n\
             CREATE TABLE IF NOT EXISTS promotion_records (\n\
             promotion_id TEXT PRIMARY KEY,\n\
             worker_id TEXT NOT NULL,\n\
             from_rank INTEGER NOT NULL,\n\
             to_rank INTEGER NOT NULL,\n\
             timestamp TEXT NOT NULL,\n\
             evidence_id TEXT NOT NULL REFERENCES worker_competence_evidence(evidence_id),\n\
             rationale TEXT NOT NULL,\n\
             evaluator_id TEXT\n\
             );\n\
             \n\
             CREATE TABLE IF NOT EXISTS promotion_thresholds (\n\
             target_rank INTEGER PRIMARY KEY,\n\
             minimum_tasks INTEGER NOT NULL DEFAULT 50,\n\
             minimum_success_rate REAL NOT NULL DEFAULT 0.85,\n\
             minimum_decisions INTEGER NOT NULL DEFAULT 10,\n\
             minimum_accuracy_rate REAL NOT NULL DEFAULT 0.80,\n\
             evidence_window_hours INTEGER NOT NULL DEFAULT 720\n\
             );\n\
             \n\
             CREATE TABLE IF NOT EXISTS worker_ranks (\n\
             worker_id TEXT PRIMARY KEY,\n\
             rank INTEGER NOT NULL DEFAULT 0\n\
             );\n\
             \n\
             INSERT OR IGNORE INTO promotion_thresholds (target_rank, minimum_tasks, minimum_success_rate)\n\
             VALUES (1, 50, 0.85);\n\
             \n\
             INSERT OR IGNORE INTO promotion_thresholds (target_rank, minimum_tasks, minimum_success_rate, minimum_decisions, minimum_accuracy_rate)\n\
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
        evidence::record_task_completion(&self.db, worker_id, success)
    }

    /// Record a decision outcome as evidence.
    pub fn record_decision(&self, worker_id: &str, accurate: bool) -> Result<(), rusqlite::Error> {
        evidence::record_decision(&self.db, worker_id, accurate)
    }

    /// Evaluate whether a worker is eligible for promotion to a target rank.
    pub fn evaluate_promotion(
        &self,
        worker_id: &str,
        target_rank: u8,
    ) -> Result<PromotionEvaluation, rusqlite::Error> {
        let current_rank = self.get_worker_rank(worker_id)?;
        evaluation::evaluate_promotion(&self.db, worker_id, current_rank, target_rank)
    }

    /// Execute a promotion for an eligible worker.
    pub fn execute_promotion(
        &self,
        worker_id: &str,
        target_rank: u8,
        rationale: &str,
    ) -> Result<PromotionRecord, rusqlite::Error> {
        let current_rank = self.get_worker_rank(worker_id)?;

        let record = evaluation::execute_promotion(
            &self.db,
            worker_id,
            current_rank,
            target_rank,
            rationale,
        )?;

        // Update worker rank
        self.set_worker_rank(worker_id, target_rank)?;

        Ok(record)
    }

    /// Get promotion history for a worker.
    pub fn get_promotion_history(
        &self,
        worker_id: &str,
    ) -> Result<Vec<PromotionRecord>, rusqlite::Error> {
        let mut stmt = self.db.prepare(
            "SELECT promotion_id, worker_id, from_rank, to_rank, timestamp, evidence_id, rationale, evaluator_id\n\
             FROM promotion_records WHERE worker_id = ? ORDER BY timestamp DESC",
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
    pub fn get_worker_metrics(
        &self,
        worker_id: &str,
    ) -> Result<WorkerCompetenceEvidence, rusqlite::Error> {
        evidence::aggregate_evidence(&self.db, worker_id, 720) // Last 30 days by default
    }
}
