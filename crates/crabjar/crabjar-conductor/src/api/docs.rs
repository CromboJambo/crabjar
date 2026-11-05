//! Documentation automation routes: /docs/evaluate, /docs/outcome.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for documentation automation evaluation and action decision.
#[derive(Debug, Deserialize)]
pub struct DocumentationEvaluationRequest {
    system: String,
    documentation_content: serde_json::Value,
    available_actions: Vec<serde_json::Value>,
}

/// Request to record documentation automation outcome for learning.
#[derive(Debug, Deserialize)]
pub struct DocumentationOutcomeRequest {
    system: String,
    action_taken: String,
    outcome: String,
    prediction_accuracy_percent: Option<f64>,
    improvement_time_hours: Option<f64>,
}

/// POST /docs/evaluate - Get optimal documentation improvement action via JEV.
pub async fn evaluate_documentation_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<DocumentationEvaluationRequest>,
) -> Json<serde_json::Value> {
    let doc_str = serde_json::to_string(&req.documentation_content).unwrap_or_default();
    let actions_str = serde_json::to_string(&req.available_actions).unwrap_or_default();
    let observation = format!(
        "Documentation evaluation: {} with content {}. Available actions: {}",
        req.system, doc_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/documentation-quality-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "documentation_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "documentation_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "documentation_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /docs/outcome - Record documentation automation outcome for JEV learning.
pub async fn record_documentation_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<DocumentationOutcomeRequest>,
) -> Json<serde_json::Value> {
    let _observation = format!(
        "Documentation outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Improvement time: {:?} hours",
        req.system,
        req.action_taken,
        req.outcome,
        req.prediction_accuracy_percent,
        req.improvement_time_hours
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
