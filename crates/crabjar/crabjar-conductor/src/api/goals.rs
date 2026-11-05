//! Goal management routes: submit and retrieve goals.

use crate::api::ConductorState;
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use tracing::info;

/// POST /goals - Submit a new goal for the conductor to orchestrate.
pub async fn submit_goal(
    State(conductor): State<ConductorState>,
    body: Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let description = body["description"]
        .as_str()
        .unwrap_or("unnamed goal")
        .to_string();

    info!(description = %description, "Submitting goal");

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.submit_goal(description)
    })
    .await;

    match result {
        Ok(Ok(goal)) => Json(json!({
            "status": "accepted",
            "goal_id": goal.id,
            "description": goal.description
        })),
        Ok(Err(e)) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}

/// GET /goals/{id} - Get the status of a specific goal.
pub async fn get_goal(
    State(conductor): State<ConductorState>,
    Path(goal_id): Path<String>,
) -> Json<serde_json::Value> {
    let goal_id_for_response = goal_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.get_goal(&goal_id)
    })
    .await;

    match result {
        Ok(Ok(Some(goal))) => Json(json!({
            "goal_id": goal.id,
            "description": goal.description,
            "status": goal.status,
            "created_at": goal.created_at.to_rfc3339()
        })),
        Ok(Ok(None)) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "not-found"
        })),
        Ok(Err(e)) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "error",
            "message": e.to_string()
        })),
        Err(e) => Json(json!({
            "goal_id": goal_id_for_response,
            "status": "error",
            "message": format!("task join error: {}", e)
        })),
    }
}
