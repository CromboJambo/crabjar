//! HTTP API endpoints for the conductor service.

use axum::{Router, routing::{get, post}, Json};
use serde_json::json;
use crate::conductor::Conductor;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug, warn, error};

pub type ConductorState = Arc<Mutex<Conductor>>;

pub fn routes(conductor: ConductorState) -> Router {
    let app = Router::new()
        // Goals
        .route("/goals", post(submit_goal))
        .route("/goals/{id}", get(get_goal))
        // Workers
        .route("/workers", get(list_workers))
        .route("/workers/register", post(register_worker))
        .route("/workers/{id}/heartbeat", post(worker_heartbeat))
        // Tasks
        .route("/tasks/for-worker/{worker_id}", get(tasks_for_worker))
        .route("/tasks/{id}/result", post(report_task_result))
        .with_state(Arc::clone(&conductor));
    
    // Add request logging middleware
    app.layer(axum::middleware::from_fn(log_requests))
}

async fn log_requests(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();
    
    debug!(method = %method, path = %path, "Incoming request");
    
    let resp = next.run(req).await;
    
    let elapsed = start.elapsed();
    info!(method = %method, path = %path, status = resp.status().as_u16(), elapsed_ms = elapsed.as_millis(), "Request completed");
    
    resp
}

async fn submit_goal(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let description = body["description"].as_str().unwrap_or("unnamed goal").to_string();

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

async fn register_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unknown").to_string();
    let capabilities: Vec<String> = body["capabilities"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.register_worker(name, capabilities)
    }).await;

    match result {
        Ok(Ok(worker)) => Json(json!({
            "status": "registered",
            "worker_id": worker.id,
            "name": worker.name
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

async fn worker_heartbeat(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let worker_id = id.0;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.worker_heartbeat(&worker_id)
    }).await;

    match result {
        Ok(Ok(())) => Json(json!({ "status": "ok" })),
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

async fn tasks_for_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let worker_id = id.0;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.tasks_for_worker(&worker_id)
    }).await;

    match result {
        Ok(Ok(tasks)) => Json(json!({
            "tasks": tasks.iter().map(|t| json!({
                "id": t.id,
                "goal_id": t.goal_id,
                "command": t.command,
                "status": t.status
            })).collect::<Vec<_>>()
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

async fn report_task_result(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let task_id = id.0;
    let status = body["status"].as_str().unwrap_or("completed").to_string();
    let result_text = body["result"].as_str().map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        match status.as_str() {
            "failed" => {
                let error = result_text.unwrap_or_default();
                c.report_task_failed(&task_id, error)
            }
            _ => c.report_task_complete(&task_id, result_text),
        }
    }).await;

    match result {
        Ok(Ok(())) => Json(json!({ "status": "recorded" })),
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
