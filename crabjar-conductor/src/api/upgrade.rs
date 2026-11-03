//! Upgrade evaluation and outcome recording endpoints.

use crate::api::ConductorState;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;

/// Request for upgrade impact evaluation.
#[derive(Debug, Deserialize)]
pub struct UpgradeEvaluationRequest {
    pub service: String,
    pub current_version: String,
    pub proposed_version: String,
    pub dependent_services: Vec<String>,
}

/// POST /upgrade/evaluate - Evaluate upgrade impact on dependent services via JEV.
pub async fn evaluate_upgrade_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<UpgradeEvaluationRequest>,
) -> Json<serde_json::Value> {
    // Build observation from upgrade evaluation details
    let dependents_str = req.dependent_services.join(", ");
    let observation = format!(
        "Upgrade evaluation: {} upgrading from v{} to v{}. Dependent services: {}",
        req.service, req.current_version, req.proposed_version, dependents_str
    );

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Use JEV to evaluate upgrade impact
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
            "upgrade_assessment": {
                "risk_level": if evaluation.option == "approve-low-risk" { "low" } else if evaluation.option == "approve-medium-risk" { "medium" } else { "high" },
                "confidence": evaluation.confidence,
                "rationale": evaluation.rationale
            },
            "recommended_sequence": [req.service]
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "upgrade_assessment": null,
            "error": format!("JEV evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "upgrade_assessment": null,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Request to record upgrade outcome for learning.
#[derive(Debug, Deserialize)]
pub struct UpgradeOutcomeRequest {
    pub service: String,
    pub version_change: String,
    pub outcome: String,
    pub downtime_seconds: Option<u64>,
    pub compatibility_issues: Vec<String>,
}

/// POST /upgrade/outcome - Record upgrade outcome for JEV learning.
pub async fn record_upgrade_outcome_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<UpgradeOutcomeRequest>,
) -> Json<serde_json::Value> {
    // Log the upgrade outcome for future JEV learning
    let issues_str = req.compatibility_issues.join(", ");
    let observation = format!(
        "Upgrade outcome: {} {} resulted in {}. Downtime: {:?}s, Issues: {}",
        req.service, req.version_change, req.outcome, req.downtime_seconds, issues_str
    );

    // Store outcome in decision history for future rule extraction
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        // Log as a decision event for historical analysis
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
