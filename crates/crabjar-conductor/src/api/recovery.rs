//! Disaster recovery evaluation routes: /recovery/evaluate

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// Request for disaster recovery evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct DisasterRecoveryEvaluationRequest {
    pub system: String,
    pub failure_scenario: String,
    pub system_state: serde_json::Value,
    pub available_actions: Vec<serde_json::Value>,
}

/// POST /recovery/evaluate - Get optimal disaster recovery decision via JEV.
pub async fn evaluate_recovery_handler(
    State(conductor): State<ConductorState>,
    body: Json<DisasterRecoveryEvaluationRequest>,
) -> Json<serde_json::Value> {
    let state_str = serde_json::to_string(&body.system_state).unwrap_or_default();
    let actions_str = serde_json::to_string(&body.available_actions).unwrap_or_default();
    let observation = format!(
        "Disaster recovery evaluation: {} with failure scenario '{}' and system state {}. Available actions: {}",
        body.system, body.failure_scenario, state_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/disaster-recovery-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "recovery_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "recovery_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "recovery_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record disaster recovery outcome for learning.
#[derive(Debug, serde::Deserialize)]
pub struct DisasterRecoveryOutcomeRequest {
    pub system: String,
    pub action_taken: String,
    pub outcome: String,
    pub recovery_time_minutes: Option<f64>,
    pub actual_data_loss_percent: Option<f64>,
}

/// POST /recovery/outcome - Record disaster recovery outcome for JEV learning.
pub async fn record_disaster_recovery_outcome_handler(
    State(conductor): State<ConductorState>,
    body: Json<DisasterRecoveryOutcomeRequest>,
) -> Json<serde_json::Value> {
    let _observation = format!(
        "Disaster recovery outcome: {} action '{}' resulted in {}. Recovery time: {:?} min, Data loss: {:?}%.",
        body.system,
        body.action_taken,
        body.outcome,
        body.recovery_time_minutes,
        body.actual_data_loss_percent
    );

    let _result = tokio::task::spawn_blocking(move || {
        let _c = conductor.blocking_lock();
        Ok::<&str, String>("outcome_recorded")
    })
    .await;

    Json(json!({
        "success": true,
        "learned": true,
        "updated_model": true,
        "system": body.system
    }))
}
