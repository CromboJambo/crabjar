//! HTTP API endpoints for the conductor service.

use axum::{Router, routing::{get, post}, Json};
use serde_json::json;

pub fn routes() -> Router {
    Router::new()
        .route("/goals", post(submit_goal))
        .route("/goals/{id}", get(get_goal))
        .route("/workers", get(list_workers))
}

async fn submit_goal(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let description = body["description"].as_str().unwrap_or("unnamed goal");
    Json(json!({
        "status": "accepted",
        "goal_id": "pending-implementation"
    }))
}

async fn get_goal(id: axum::extract::Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "goal_id": id.0,
        "status": "not-found"
    }))
}

async fn list_workers() -> Json<serde_json::Value> {
    Json(json!({
        "workers": []
    }))
}
