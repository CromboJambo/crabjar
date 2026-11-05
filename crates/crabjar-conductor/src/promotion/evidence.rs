//! Evidence collection and aggregation for worker promotion.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use super::types::WorkerCompetenceEvidence;

/// Record a task completion as evidence.
pub fn record_task_completion(
    db: &Connection,
    worker_id: &str,
    success: bool,
) -> Result<(), rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    let evidence_id = format!("ev_{}", Uuid::new_v4());

    // Create or update today's evidence record for this worker
    db.execute(
        "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp, task_completions, success_count, failure_count)\n\
         VALUES (?, ?, ?, 1, ?, ?)",
        params![evidence_id, worker_id, now, i32::from(success), i32::from(!success)],
    )?;

    Ok(())
}

/// Record a decision outcome as evidence.
pub fn record_decision(
    db: &Connection,
    worker_id: &str,
    accurate: bool,
) -> Result<(), rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    let evidence_id = format!("ev_{}", Uuid::new_v4());

    db.execute(
        "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp, decisions_made, accurate_decisions)\n\
         VALUES (?, ?, ?, 1, ?)",
        params![evidence_id, worker_id, now, i32::from(accurate)],
    )?;

    Ok(())
}

/// Aggregate evidence for a worker within a time window.
pub fn aggregate_evidence(
    db: &Connection,
    worker_id: &str,
    window_hours: u32,
) -> Result<WorkerCompetenceEvidence, rusqlite::Error> {
    let cutoff = Utc::now()
        .checked_sub_signed(Duration::hours(i64::from(window_hours)))
        .unwrap_or(Utc::now())
        .to_rfc3339();

    let row = db.query_row(
        "SELECT COALESCE(SUM(task_completions), 0), COALESCE(SUM(success_count), 0),\n\
         COALESCE(SUM(failure_count), 0), COALESCE(SUM(decisions_made), 0),\n\
         COALESCE(SUM(accurate_decisions), 0), COALESCE(SUM(knowledge_contributions), 0)\n\
         FROM worker_competence_evidence\n\
         WHERE worker_id = ? AND timestamp >= ?",
        params![worker_id, cutoff],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        },
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
