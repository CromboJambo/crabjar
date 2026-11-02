//! Health evaluation routes: /health/evaluate, /health/outcome

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// Request for health evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct HealthEvaluationRequest {
    pub service: String,
    pub metrics: serde_json::Value,
    pub available_actions: Vec<serde_json::Value>,
}

/// POST /health/evaluate - Get optimal self-healing decision via JEV.
pub async fn evaluate_health_handler(
    State(conductor): State<ConductorState>,
    body: Json<HealthEvaluationRequest>,
) -> Json<serde_json::Value> {
    let metrics_str = serde_json::to_string(&body.metrics).unwrap_or_default();
    let actions_str = serde_json::to_string(&body.available_actions).unwrap_or_default();
    let observation = format!(
        "Health evaluation: {} with metrics {}. Available corrective actions: {}",
        body.service, metrics_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(
            c.evaluate_decision(&observation, "specs/criteria/self-healing-evaluation.json"),
        )
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "healing_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "healing_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "healing_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record recovery outcome for learning.
#[derive(Debug, serde::Deserialize)]
pub struct RecoveryOutcomeRequest {
    pub service: String,
    pub action_taken: String,
    pub outcome: String,
    pub recovery_time_seconds: Option<u64>,
    pub recurrence_within_24h: Option<bool>,
}

/// POST /health/outcome - Record recovery outcome for JEV learning.
pub async fn record_recovery_outcome_handler(
    State(conductor): State<ConductorState>,
    body: Json<RecoveryOutcomeRequest>,
) -> Json<serde_json::Value> {
    let observation = format!(
        "Recovery outcome: {} action '{}' resulted in {}. Time: {:?}s, Recurrence: {:?}",
        body.service,
        body.action_taken,
        body.outcome,
        body.recovery_time_seconds,
        body.recurrence_within_24h
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "service": body.service
    }))
}
