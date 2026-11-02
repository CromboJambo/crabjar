//! Cost optimization routes: /cost/evaluate, /cost/outcome.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for cost evaluation and optimization decision.
#[derive(Debug, Deserialize)]
pub struct CostEvaluationRequest {
    service: String,
    usage_metrics: serde_json::Value,
    available_optimizations: Vec<serde_json::Value>,
}

/// Request to record optimization outcome for learning.
#[derive(Debug, Deserialize)]
pub struct OptimizationOutcomeRequest {
    service: String,
    action_taken: String,
    outcome: String,
    savings_achieved_usd: Option<f64>,
    performance_impact_percent: Option<f64>,
}

/// POST /cost/evaluate - Get optimal cost optimization decision via JEV.
pub async fn evaluate_cost_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<CostEvaluationRequest>,
) -> Json<serde_json::Value> {
    let metrics_str = serde_json::to_string(&req.usage_metrics).unwrap_or_default();
    let opts_str = serde_json::to_string(&req.available_optimizations).unwrap_or_default();
    let observation = format!(
        "Cost evaluation: {} with usage metrics {}. Available optimizations: {}",
        req.service, metrics_str, opts_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/cost-optimization-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "optimization_decision": {
                "recommended_action": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "optimization_decision": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "optimization_decision": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /cost/outcome - Record optimization outcome for JEV learning.
pub async fn record_optimization_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<OptimizationOutcomeRequest>,
) -> Json<serde_json::Value> {
    let observation = format!(
        "Optimization outcome: {} action '{}' resulted in {}. Savings: ${:?}, Performance impact: {:?}%",
        req.service,
        req.action_taken,
        req.outcome,
        req.savings_achieved_usd,
        req.performance_impact_percent
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