//! Decision evaluation routes: evaluate, poll status, list capabilities.

use crate::api::ConductorState;
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

/// In-memory store for background evaluations (eval_id -> status JSON).
static BG_EVALUATIONS: OnceLock<Arc<Mutex<std::collections::HashMap<String, serde_json::Value>>>> =
    OnceLock::new();

fn bg_evaluations() -> Arc<Mutex<std::collections::HashMap<String, serde_json::Value>>> {
    BG_EVALUATIONS
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone()
}

/// GET /decisions/poll/{id} - Poll the status of a background evaluation.
pub async fn get_evaluation_status(Path(eval_id): Path<String>) -> Json<serde_json::Value> {
    let store = bg_evaluations();
    let map = match store.lock() {
        Ok(guard) => guard,
        Err(_) => return Json(json!({"success": false, "error": "Lock poisoned"})),
    };

    match map.get(&eval_id) {
        Some(status_json) => Json(status_json.clone()),
        None => Json(json!({
            "success": false,
            "error": format!("Evaluation not found: {}", eval_id)
        })),
    }
}

/// GET /decisions/capabilities - List available decision evaluation capabilities.
pub async fn list_decision_capabilities() -> Json<serde_json::Value> {
    let capabilities = [
        ("structural-condition", "Evaluate structural conditions directly against code region"),
        ("embedding-classification", "Classify using JEV embedding similarity matching"),
        ("hybrid", "Try structural first, fall back to embedding-based evaluation"),
    ];

    Json(json!({
        "success": true,
        "capabilities": capabilities.iter().map(|(id, desc)| {
            json!({"id": id, "description": desc})
        }).collect::<Vec<_>>()
    }))
}

/// POST /decisions/evaluate - Evaluate a decision via JEV semantic matching.
pub async fn evaluate_decision_handler(
    State(conductor): State<ConductorState>,
    Json(req): Json<EvaluateDecisionRequest>,
) -> Json<serde_json::Value> {
    let observation = req.observation.clone();
    let criteria_file = req.criteria_file.clone();

    // If background flag is set, spawn async task and return ID immediately
    if req.background {
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
            let eval_store = bg_evaluations();
            let mut store = eval_store.lock().unwrap();
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
            "poll_url": format!("/decisions/poll/{}", eval_id)
        }));
    }

    // Synchronous evaluation
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let rt = tokio::runtime::Handle::current();
        rt.block_on(c.evaluate_decision(&observation, &criteria_file))
    })
    .await;

    match result {
        Ok(Ok(decision)) => Json(json!({
            "success": true,
            "evaluation": {
                "option": decision.option,
                "rationale": decision.rationale,
                "confidence": decision.confidence,
                "model": null
            }
        })),
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

/// Request for JEV decision evaluation.
#[derive(Debug, serde::Deserialize)]
pub struct EvaluateDecisionRequest {
    /// The observation to evaluate (code change, PR description, etc.).
    pub observation: String,
    /// Path to the criteria file defining options and thresholds.
    pub criteria_file: String,
    /// If true, evaluate asynchronously and return an ID for polling.
    #[serde(default)]
    pub background: bool,
}
