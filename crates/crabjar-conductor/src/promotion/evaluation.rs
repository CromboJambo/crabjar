//! Promotion eligibility evaluation logic.

use chrono::Utc;
use rusqlite::{Connection, params};
use uuid::Uuid;

use super::evidence::{aggregate_evidence, record_task_completion};
use super::types::{PromotionEvaluation, PromotionThresholdConfig, ThresholdCheck};

/// Get threshold configuration for a target rank.
fn get_thresholds(
    db: &Connection,
    target_rank: u8,
) -> Result<PromotionThresholdConfig, rusqlite::Error> {
    let row = db.query_row(
        "SELECT minimum_tasks, minimum_success_rate, minimum_decisions, minimum_accuracy_rate, evidence_window_hours\n\
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
        Ok((tasks, success_rate, decisions, accuracy_rate, window)) => {
            Ok(PromotionThresholdConfig {
                target_rank,
                minimum_tasks: tasks as u32,
                minimum_success_rate: success_rate,
                minimum_decisions: decisions as u32,
                minimum_accuracy_rate: accuracy_rate,
                evidence_window_hours: window as u32,
            })
        }
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
    db: &Connection,
    worker_id: &str,
    current_rank: u8,
    target_rank: u8,
) -> Result<PromotionEvaluation, rusqlite::Error> {
    let thresholds = get_thresholds(db, target_rank)?;
    let evidence = aggregate_evidence(db, worker_id, thresholds.evidence_window_hours)?;

    // Evaluate each threshold criterion
    let mut checks = Vec::new();

    // Task completion volume
    let tasks_met = evidence.task_completions >= thresholds.minimum_tasks;
    checks.push(ThresholdCheck {
        criterion: "minimum_tasks".to_string(),
        required: f64::from(thresholds.minimum_tasks),
        actual: f64::from(evidence.task_completions),
        met: tasks_met,
    });

    // Success rate
    let success_rate = if evidence.task_completions > 0 {
        f64::from(evidence.success_count) / f64::from(evidence.task_completions)
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
            required: f64::from(thresholds.minimum_decisions),
            actual: f64::from(evidence.decisions_made),
            met: decisions_met,
        });

        // Decision accuracy
        let accuracy_rate = if evidence.decisions_made > 0 {
            f64::from(evidence.accurate_decisions) / f64::from(evidence.decisions_made)
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
    db: &Connection,
    worker_id: &str,
    current_rank: u8,
    target_rank: u8,
    rationale: &str,
) -> Result<super::types::PromotionRecord, rusqlite::Error> {
    if current_rank >= target_rank {
        return Err(rusqlite::Error::InvalidParameterName(
            "Already at or above target rank".to_string(),
        ));
    }

    // Evaluate eligibility first
    let evaluation = evaluate_promotion(db, worker_id, current_rank, target_rank)?;
    if !evaluation.eligible {
        return Err(rusqlite::Error::InvalidParameterName(
            "Not eligible for promotion".to_string(),
        ));
    }

    // Create an actual evidence record for this promotion decision
    let now = Utc::now().to_rfc3339();
    let evidence_id = format!("pr_ev_{}", Uuid::new_v4());
    db.execute(
        "INSERT INTO worker_competence_evidence (evidence_id, worker_id, timestamp) VALUES (?, ?, ?)",
        params![evidence_id, worker_id, now],
    )?;

    let promotion_id = format!("pr_{}", Uuid::new_v4());

    // Record the promotion
    db.execute(
        "INSERT INTO promotion_records (promotion_id, worker_id, from_rank, to_rank, timestamp, evidence_id, rationale)\n\
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

    Ok(super::types::PromotionRecord {
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
