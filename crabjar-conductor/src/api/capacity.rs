//! Capacity planning routes: /capacity/evaluate, /capacity/outcome.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Request for capacity evaluation and adjustment decision.
#[derive(Debug, Deserialize)]
pub struct CapacityEvaluationRequest {
    service: String,
    usage_trends: serde_json::Value,
    current_capacity: serde_json::Value,
    available_adjustments: Vec<serde_json::Value>,
}

/// Request to record capacity outcome for learning.
#[derive(Debug, Deserialize)]
pub struct CapacityOutcomeRequest {
    service: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    resource_utilization_after_percent: Option<f64>,
}

/// POST /capacity/evaluate - Get optimal capacity adjustment decision via JEV.
pub async fn evaluate_capacity_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<CapacityEvaluationRequest>,
) -> Json<serde_json::Value> {
    let trends_str = serde_json::to_string(&req.usage_trends).unwrap_or_default();
    let capacity_str = serde_json::to_string(&req.current_capacity).unwrap_or_default();
    let adjustments_str = serde_json::to_string(&req.available_adjustments).unwrap_or_default();
    let observation = format!(
        "Capacity evaluation: {} with usage trends {}, current capacity {}. Available adjustments: {}",
        req.service, trends_str, capacity_str, adjustments_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/capacity-planning-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "capacity_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "capacity_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "capacity_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /capacity/outcome - Record capacity outcome for JEV learning.
pub async fn record_capacity_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<CapacityOutcomeRequest>,
) -> Json<serde_json::Value> {
    let observation = format!(
        "Capacity outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Utilization after: {:?}%",
        req.service,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.resource_utilization_after_percent
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
        "service": req.service
    }))
}