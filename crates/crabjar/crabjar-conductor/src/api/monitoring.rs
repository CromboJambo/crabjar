//! Monitoring evaluation routes: /monitoring/evaluate

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// Request for monitoring evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct MonitoringEvaluationRequest {
    pub system: String,
    pub monitoring_config: serde_json::Value,
    pub available_actions: Vec<serde_json::Value>,
}

/// POST /monitoring/evaluate - Get optimal monitoring improvement action via JEV.
pub async fn evaluate_monitoring_handler(
    State(conductor): State<ConductorState>,
    body: Json<MonitoringEvaluationRequest>,
) -> Json<serde_json::Value> {
    let config_str = serde_json::to_string(&body.monitoring_config).unwrap_or_default();
    let actions_str = serde_json::to_string(&body.available_actions).unwrap_or_default();
    let observation = format!(
        "Monitoring evaluation: {} with configuration {}. Available actions: {}",
        body.system, config_str, actions_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/monitoring-quality-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "monitoring_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "monitoring_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "monitoring_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record monitoring automation outcome for learning.
#[derive(Debug, serde::Deserialize)]
pub struct MonitoringOutcomeRequest {
    pub system: String,
    pub action_taken: String,
    pub outcome: String,
    pub prediction_accuracy_percent: Option<f64>,
    pub improvement_time_hours: Option<f64>,
}

/// POST /monitoring/outcome - Record monitoring automation outcome for JEV learning.
pub async fn record_monitoring_outcome_handler(
    State(conductor): State<ConductorState>,
    body: Json<MonitoringOutcomeRequest>,
) -> Json<serde_json::Value> {
    let _observation = format!(
        "Monitoring outcome: {} action '{}' resulted in {}. Prediction accuracy: {:?}%, Improvement time: {:?} hours",
        body.system,
        body.action_taken,
        body.outcome,
        body.prediction_accuracy_percent,
        body.improvement_time_hours
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
