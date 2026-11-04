//! Interface optimization evaluation routes: /interface/propose

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde_json::json;

/// Request for interface optimization proposal evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct InterfaceOptimizationRequest {
    pub optimization_type: String,
    pub source_component: String,
    pub target_component: String,
    pub current_format: String,
    pub proposed_format: String,
    pub justification: String,
}

/// POST /interface/propose - Evaluate an interface optimization proposal via JEV.
pub async fn propose_interface_optimization_handler(
    State(conductor): State<ConductorState>,
    body: Json<InterfaceOptimizationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from proposal details
    let observation = format!(
        "Interface optimization proposal: {} wants to change interface to {} from {} to {}. Justification: {}",
        body.source_component,
        body.target_component,
        body.current_format,
        body.proposed_format,
        body.justification
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate interface optimization impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/interface-optimization-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "evaluation": {
                "option": evaluation.option,
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            },
            "deployment_plan": {
                "steps": [],
                "rollback_steps": [],
                "affected_endpoints": []
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "evaluation": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}
