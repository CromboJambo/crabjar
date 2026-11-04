//! Decision evaluation routes: /evaluations/decision

use crate::api::{ConductorState};
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use uuid::Uuid;

/// Request for decision evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct EvaluateDecisionRequest {
    pub observation: String,
    pub criteria_file: String,
    /// If true, run evaluation asynchronously and return an ID for polling.
    #[serde(default)]
    pub background: bool,
}

/// POST /evaluations/decision - Evaluate a decision using JEV.
pub async fn evaluate_decision_handler(
    State(conductor): State<ConductorState>,
    body: Json<EvaluateDecisionRequest>,
) -> Json<serde_json::Value> {
    let observation = body.observation.clone();
    let criteria_file = body.criteria_file.clone();

    // If background flag is set, spawn async task and return ID immediately
    if body.background {
        let eval_id = Uuid::new_v4().to_string();
        let eval_id_for_closure = eval_id.clone();
        let conductor_clone = conductor.clone();

        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let c = conductor_clone.blocking_lock();
                let rt = tokio::runtime::Handle::current();
                rt.block_on(c.evaluate_decision(&observation, &criteria_file))
            })
            .await;

            // Store result for polling
            let mut store = crate::api::bg_evaluations().lock().await;
            match result {
                Ok(Ok(decision)) => {
                    store.insert(
                        eval_id_for_closure.clone(),
                        json!({
                            "status": "completed",
                            "success": true,
                            "option": decision.option,
                            "confidence": decision.confidence,
                            "model": null,
                            "rationale": decision.rationale
                        }),
                    );
                }
                Ok(Err(e)) => {
                    store.insert(
                        eval_id_for_closure.clone(),
                        json!({
                            "status": "completed",
                            "success": false,
                            "option": "error",
                            "confidence": 0.0,
                            "model": null,
                            "rationale": format!("Evaluation failed: {}", e)
                        }),
                    );
                }
                Err(e) => {
                    store.insert(
                        eval_id_for_closure,
                        json!({
                            "status": "completed",
                            "success": false,
                            "option": "error",
                            "confidence": 0.0,
                            "model": null,
                            "rationale": format!("Task join error: {}", e)
                        }),
                    );
                }
            }
        });

        return Json(json!({
            "success": true,
            "evaluation_id": eval_id,
            "status": "pending",
            "message": "Evaluation started in background. Poll /decide/evaluate/{id} for result."
        }));
    }

    // Synchronous evaluation (default)
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, &criteria_file))
    })
    .await;

    match result {
        Ok(Ok(decision)) => Json(json!({
            "success": true,
            "option": decision.option,
            "confidence": decision.confidence,
            "model": null,
            "rationale": decision.rationale
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "option": "error",
            "confidence": 0.0,
            "model": null,
            "rationale": format!("Evaluation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "option": "error",
            "confidence": 0.0,
            "model": null,
            "rationale": format!("Task join error: {}", e)
        })),
    }
}

/// GET /decide/evaluate/{id} - Poll status of a background evaluation.
pub async fn get_evaluation_status(
    Path(eval_id): Path<String>,
) -> Json<serde_json::Value> {
    let store = crate::api::bg_evaluations().lock().await;

    match store.get(&eval_id) {
        Some(result) => {
            let status = &result["status"];
            if status == "completed" {
                Json(json!({
                    "success": true,
                    "evaluation_id": eval_id,
                    "status": status,
                    "result": result
                }))
            } else {
                Json(json!({
                    "success": true,
                    "evaluation_id": eval_id,
                    "status": status,
                    "result": null
                }))
            }
        },
        None => Json(json!({
            "success": false,
            "evaluation_id": eval_id,
            "error": "Evaluation not found or expired"
        })),
    }
}

/// GET /decisions/capabilities - List available decision-making capabilities.
pub async fn list_decision_capabilities() -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "capabilities": [
            {
                "id": "jev-structural",
                "name": "JEV Structural Evaluation",
                "description": "Evaluate code regions against JEV criteria using embedding similarity"
            },
            {
                "id": "decision-postprocess",
                "name": "Decision Post-Processing",
                "description": "Validate and refine JEV decisions with structural rule checks"
            },
            {
                "id": "feedback-learning",
                "name": "Feedback Learning",
                "description": "Record feedback on decisions for future improvement"
            }
        ]
    }))
}

/// GET /audit/log - Query the decision audit log.
pub async fn query_audit_log() -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "entries": []
    }))
}
