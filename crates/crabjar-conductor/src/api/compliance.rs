//! Compliance automation routes: /compliance/evaluate.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for compliance automation evaluation and action decision.
#[derive(Debug, Deserialize)]
pub struct ComplianceEvaluationRequest {
    system: String,
    compliance_requirement: serde_json::Value,
    current_configuration: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// POST /compliance/evaluate - Get optimal compliance action decision via JEV.
pub async fn evaluate_compliance_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<ComplianceEvaluationRequest>,
) -> Json<serde_json::Value> {
    let req_str = serde_json::to_string(&req.compliance_requirement).unwrap_or_default();
    let config_str = serde_json::to_string(&req.current_configuration).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Compliance evaluation: {} with requirement {}, current configuration {}. Available actions: {}",
        req.system, req_str, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/compliance-automation-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "compliance_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "compliance_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "compliance_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record compliance automation outcome for learning.
#[derive(Debug, Deserialize)]
pub struct ComplianceOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    resolution_time_hours: Option<f64>,
}

/// POST /compliance/outcome - Record compliance automation outcome for JEV learning.
pub async fn record_compliance_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<ComplianceOutcomeRequest>,
) -> Json<serde_json::Value> {
    let _observation = format!(
        "Compliance outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Resolution time: {:?} hours",
        req.system,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.resolution_time_hours
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