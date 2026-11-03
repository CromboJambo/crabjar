//! Decision evaluation routes: /evaluations/decision

use crate::api::{ConductorState, bg_evaluations};
use axum::{
    Json,
    extract::State,
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
            let mut store = bg_evaluations().lock().unwrap();
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
