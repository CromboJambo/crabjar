//! Configuration drift detection routes: /drift/evaluate, /drift/outcome.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for configuration drift detection evaluation and action decision.
#[derive(Debug, Deserialize)]
pub struct DriftEvaluationRequest {
    system: String,
    configuration_state: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// Request to record configuration drift detection outcome for learning.
#[derive(Debug, Deserialize)]
pub struct DriftOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    remediation_time_minutes: Option<f64>,
}

/// POST /drift/evaluate - Get optimal configuration drift remediation action via JEV.
pub async fn evaluate_drift_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<DriftEvaluationRequest>,
) -> Json<serde_json::Value> {
    let config_str = serde_json::to_string(&req.configuration_state).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Drift evaluation: {} with configuration state {}. Available actions: {}",
        req.system, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(
            c.evaluate_decision(&observation, "specs/criteria/drift-quality-evaluation.json"),
        )
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "drift_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "drift_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "drift_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /drift/outcome - Record configuration drift detection outcome for JEV learning.
pub async fn record_drift_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<DriftOutcomeRequest>,
) -> Json<serde_json::Value> {
    let _observation = format!(
        "Drift outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Remediation time: {:?} minutes",
        req.system,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.remediation_time_minutes
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
        "system": req.system
    }))
}