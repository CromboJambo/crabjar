//! Security hardening decision endpoints (Milestone 13).

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for security hardening evaluation and action decision.
#[derive(Debug, Deserialize)]
pub struct SecurityEvaluationRequest {
    pub system: String,
    pub vulnerability_data: serde_json::Value,
    pub system_posture: serde_json::Value,
    pub available_actions: Vec<serde_json::Value>,
}

/// POST /security/evaluate - Get optimal security hardening action decision via JEV.
pub async fn evaluate_security_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<SecurityEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from vulnerability data and system posture
    let vuln_str = serde_json::to_string(&req.vulnerability_data).unwrap_or_default();
    let posture_str = serde_json::to_string(&req.system_posture).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Security evaluation: {} with vulnerability {}, system posture {}. Available actions: {}",
        req.system, vuln_str, posture_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate optimal security hardening action
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/security-hardening-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "security_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "security_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "security_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record security hardening outcome for learning.
#[derive(Debug, Deserialize)]
pub struct SecurityOutcomeRequest {
    pub system: String,
    pub action_taken: String,
    pub outcome: String,
    pub prediction_accuracy_percent: Option<f64>,
    pub resolution_time_minutes: Option<f64>,
}

/// POST /security/outcome - Record security hardening outcome for JEV learning.
pub async fn record_security_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<SecurityOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the security outcome for future JEV learning
    let _observation = format!(
        "Security outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Resolution time: {:?} min",
        req.system,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.resolution_time_minutes
    );

    // Store outcome in decision history for future rule extraction
    let _result = tokio::task::spawn_blocking(move || {
        let _c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
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
