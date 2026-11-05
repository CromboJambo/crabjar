//! Evidence-based promotion API handlers.

use crate::api::ConductorState;
use axum::{Json, extract::Path};
use serde_json::json;

/// GET /`promotion/evaluate/:worker_id` - Evaluate promotion eligibility.
pub async fn evaluate_promotion_endpoint(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    worker_id: Path<String>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Default to evaluating for rank 1 (supervisor)
        c.promotion_manager().evaluate_promotion(&worker_id, 1)
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "eligible": evaluation.eligible,
            "current_rank": evaluation.current_rank,
            "target_rank": evaluation.target_rank,
            "evidence_summary": {
                "task_completions": evaluation.evidence_summary.task_completions,
                "success_count": evaluation.evidence_summary.success_count,
                "failure_count": evaluation.evidence_summary.failure_count,
                "decisions_made": evaluation.evidence_summary.decisions_made,
                "accurate_decisions": evaluation.evidence_summary.accurate_decisions
            },
            "thresholds_met": evaluation.thresholds_met.iter().map(|c| json!({
                "criterion": c.criterion,
                "required": c.required,
                "actual": c.actual,
                "met": c.met
            })).collect::<Vec<_>>()
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /`promotion/execute/:worker_id` - Execute promotion for eligible worker.
pub async fn execute_promotion_endpoint(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    worker_id: Path<String>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rationale = "Automatic evidence-based promotion";
        // Default to promoting to rank 1 (supervisor)
        c.promotion_manager()
            .execute_promotion(&worker_id, 1, rationale)
    })
    .await;

    match result {
        Ok(Ok(record)) => Json(json!({
            "success": true,
            "promotion_id": record.promotion_id,
            "worker_id": record.worker_id,
            "from_rank": record.from_rank,
            "to_rank": record.to_rank,
            "timestamp": record.timestamp.to_rfc3339(),
            "rationale": record.rationale
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Promotion failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// GET /`promotion/history/:worker_id` - Get promotion history for worker.
pub async fn get_promotion_history_endpoint(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    worker_id: Path<String>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.promotion_manager().get_promotion_history(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(records)) => Json(json!({
            "success": true,
            "promotions": records.iter().map(|r| json!({
                "promotion_id": r.promotion_id,
                "from_rank": r.from_rank,
                "to_rank": r.to_rank,
                "timestamp": r.timestamp.to_rfc3339(),
                "rationale": r.rationale
            })).collect::<Vec<_>>()
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("History retrieval failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// GET /`promotion/metrics/:worker_id` - Get competence metrics for worker.
pub async fn get_worker_metrics_endpoint(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    worker_id: Path<String>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.promotion_manager().get_worker_metrics(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(metrics)) => Json(json!({
            "success": true,
            "worker_id": metrics.worker_id,
            "task_completions": metrics.task_completions,
            "success_count": metrics.success_count,
            "failure_count": metrics.failure_count,
            "decisions_made": metrics.decisions_made,
            "accurate_decisions": metrics.accurate_decisions,
            "knowledge_contributions": metrics.knowledge_contributions
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Metrics retrieval failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}
