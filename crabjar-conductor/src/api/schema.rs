//! Schema co-evolution decision endpoints (Milestone 5).

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for schema change proposal evaluation.
#[derive(Debug, Deserialize)]
pub struct SchemaProposalRequest {
    pub component: String,
    pub change_type: String,
    pub target_table: String,
    pub column_name: Option<String>,
    pub column_type: Option<String>,
    pub justification: String,
}

/// POST /schema/propose - Evaluate a schema change proposal via JEV.
pub async fn propose_schema_change_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<SchemaProposalRequest>,
) -> Json<serde_json::Value> {
    // Build observation from proposal details
    let observation = format!(
        "Schema change proposal: {} wants to {} on {}. Justification: {}",
        req.component, req.change_type, req.target_table, req.justification
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate schema change impact
        let rt = tokio::runtime::Handle::current();
        rt.block_on(
            c.evaluate_decision(&observation, "specs/criteria/schema-change-evaluation.json"),
        )
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
            "migration_plan": {
                "steps": [],
                "rollback_steps": [],
                "affected_components": [req.component]
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
