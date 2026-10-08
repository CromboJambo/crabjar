//! HTTP API endpoints for the conductor service.

use axum::{Router, routing::{get, post}, Json};
use serde_json::json;
use crate::conductor::Conductor;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ConductorState = Arc<Mutex<Conductor>>;

pub fn routes(conductor: ConductorState) -> Router {
    Router::new()
        .route("/goals", post(submit_goal))
        .route("/goals/{id}", get(get_goal))
        .route("/workers", get(list_workers))
        .with_state(Arc::clone(&conductor))
}

async fn submit_goal(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let description = body["description"].as_str().unwrap_or("unnamed goal").to_string();

    // Lock the conductor to perform the operation
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.submit_goal(description)
    }).await;

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
        }))
    }
}

async fn get_goal(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let goal_id = id.0;
    let goal_id_for_response = goal_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.get_goal(&goal_id)
    }).await;

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
        }))
    }
}

async fn list_workers(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.list_workers()
    }).await;

    match result {
        Ok(workers) => Json(json!({
            "workers": workers.iter().map(|w| json!({
                "id": w.id,
                "name": w.name,
                "status": w.status,
                "capabilities": w.capabilities
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("task join error: {}", e)
        }))
    }
}