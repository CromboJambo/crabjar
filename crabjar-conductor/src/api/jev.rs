//! JEV (Just-in-Time Evaluation) routes: structural evaluation, post-processing, and feedback.

use crate::api::ConductorState;
use axum::{
    Json,
    extract::State,
};
use serde_json::json;

/// POST /jev/evaluate - Evaluate a structural code region against JEV criteria.
pub async fn evaluate_structural_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<EvaluateStructuralRequest>,
) -> Json<serde_json::Value> {
    let target = req.target;
    let criteria_path = req.criteria_path.clone();

    // First, try direct structural condition evaluation
    if let Some(result) = evaluate_structural_conditions(&target, &criteria_path) {
        return result;
    }

    // Fall back to embedding-based JEV evaluation
    let observation = target.to_observation();
    let post_criteria_path = criteria_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, &criteria_path))
    })
    .await;

    match result {
        Ok(Ok(decision)) => {
            // Phase 3: Post-process the decision for validation
            let post_process = crate::decision::postprocessor::validate_decision(
                &target,
                &decision.option,
                &post_criteria_path,
            );

            Json(json!({
                "success": true,
                "evaluation": {
                    "option": decision.option,
                    "rationale": decision.rationale,
                    "confidence": decision.confidence + post_process.confidence_adjustment,
                    "post_processed": true,
                    "validation": {
                        "valid": post_process.valid,
                        "issues": post_process.issues
                    }
                }
            }))
        }
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Decision evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// POST /jev/postprocess - Validate a JEV decision against structural rules.
pub async fn postprocess_decision_handler(
    Json(body): Json<PostProcessRequest>,
) -> Json<serde_json::Value> {
    let result = crate::decision::postprocessor::validate_decision(
        &body.target,
        &body.option_id,
        &body.criteria_path,
    );

    Json(json!({
        "success": true,
        "validation": {
            "valid": result.valid,
            "issues": result.issues,
            "confidence_adjustment": result.confidence_adjustment
        }
    }))
}

/// POST /jev/feedback - Record feedback on a JEV decision for reflection.
pub async fn record_feedback_handler(
    Json(body): Json<crate::decision::postprocessor::DecisionFeedback>,
) -> Json<serde_json::Value> {
    match crate::decision::postprocessor::record_feedback(&body) {
        Ok(()) => Json(json!({
            "success": true,
            "message": "Feedback recorded"
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}

/// Evaluate structural conditions directly against the target.
/// Returns Some(Json) if a condition matched, None to fall back to embedding evaluation.
fn evaluate_structural_conditions(
    target: &crate::decision::structural::EvaluationTarget,
    criteria_path: &str,
) -> Option<Json<serde_json::Value>> {
    // Read and parse the criteria file
    let criteria_content = match std::fs::read_to_string(criteria_path) {
        Ok(content) => content,
        Err(_) => return None, // File not found or unreadable — fall back to JEV
    };

    let criteria: serde_json::Value = match serde_json::from_str(&criteria_content) {
        Ok(value) => value,
        Err(_) => return None, // Invalid JSON — fall back to JEV
    };

    // Check each option for structural conditions
    if let Some(options) = criteria["options"].as_array() {
        for option in options {
            if let Some(condition) = option["conditions"].as_str()
                && crate::decision::condition::evaluate_condition(target, condition) {
                    return Some(Json(json!({
                        "success": true,
                        "evaluation": {
                            "option": option["id"].as_str().unwrap_or("unknown"),
                            "rationale": format!("Structural condition matched: {}", condition),
                            "confidence": 1.0,
                            "method": "structural-condition"
                        }
                    })));
                }
        }
    }

    None // No conditions matched — fall back to JEV embedding evaluation
}

/// Request for structural region evaluation via JEV.
#[derive(Debug, serde::Deserialize)]
pub struct EvaluateStructuralRequest {
    /// The code region to evaluate (tokens or source with range).
    pub target: crate::decision::structural::EvaluationTarget,
    /// Path to the JEV criteria file.
    pub criteria_path: String,
}

/// Request for post-processing a JEV decision.
#[derive(Debug, serde::Deserialize)]
pub struct PostProcessRequest {
    /// The evaluation target.
    pub target: crate::decision::structural::EvaluationTarget,
    /// The option ID that was selected.
    pub option_id: String,
    /// Path to the JEV criteria file.
    pub criteria_path: String,
}
