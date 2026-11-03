//! Cross-service dependency management routes: /dependencies/declare.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for dependency declaration validation.
#[derive(Debug, Deserialize)]
pub struct DependencyDeclarationRequest {
    service: String,
    version: String,
    dependencies: Vec<serde_json::Value>,
}

/// POST /dependencies/declare - Validate service dependency declarations via JEV.
pub async fn declare_dependencies_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<DependencyDeclarationRequest>,
) -> Json<serde_json::Value> {
    let deps_str = serde_json::to_string(&req.dependencies).unwrap_or_default();
    let observation = format!(
        "Dependency declaration: {} v{} declares dependencies: {}",
        req.service, req.version, deps_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(
            &observation,
            "specs/criteria/upgrade-impact-evaluation.json",
        ))
    })
    .await;

    match result {
        Ok(Ok(evaluation)) => Json(json!({
            "success": true,
            "validation": {
                "compatible": evaluation.option == "approve-low-risk" || evaluation.option == "approve-medium-risk",
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            }
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "validation": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "validation": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}