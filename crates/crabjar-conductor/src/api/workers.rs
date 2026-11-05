//! Worker-related API handlers.

use crate::api::ConductorState;
use axum::{Json, extract::Path};
use serde_json::json;
use tracing::info;

/// GET /workers - List all registered workers (protected).
pub async fn list_workers(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.list_workers()
    })
    .await;

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
        })),
    }
}

/// POST /workers/register - Register a new worker (public).
pub async fn register_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unknown").to_string();
    let capabilities: Vec<String> = body["capabilities"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                .collect()
        })
        .unwrap_or_default();
    let ssh_key = body["ssh_key"]
        .as_str()
        .map(std::string::ToString::to_string);

    info!(name = %name, capabilities = ?capabilities, "Worker registering");

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.register_worker(name, capabilities, ssh_key)
    })
    .await;

    match result {
        Ok(Ok((worker, token))) => Json(json!({
            "status": "registered",
            "worker_id": worker.id,
            "name": worker.name,
            "auth_token": token
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

/// POST /workers/login - Authenticate existing worker (public).
pub async fn worker_login_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("").to_string();
    let ssh_key = body["ssh_key"]
        .as_str()
        .map(std::string::ToString::to_string);

    info!(name = %name, "Worker logging in");

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.worker_login(&name, ssh_key)
    })
    .await;

    match result {
        Ok(Ok((worker, token))) => Json(json!({
            "status": "authenticated",
            "worker_id": worker.id,
            "name": worker.name,
            "auth_token": token,
            "trust_level": worker.trust_level
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

/// POST /workers/poll - Worker polls for available tasks (public).
pub async fn worker_poll_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let worker_id = body["worker_id"].as_str().unwrap_or("").to_string();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.tasks_for_worker(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(tasks)) if !tasks.is_empty() => Json(json!({
            "status": "task_available",
            "task_id": tasks[0].id,
            "goal_id": tasks[0].goal_id,
            "command": tasks[0].command
        })),
        Ok(Ok(_)) => Json(json!({
            "status": "no_tasks"
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

/// GET /tasks/for-worker/:worker_id - List tasks assigned to worker (protected).
pub async fn tasks_for_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: Path<String>,
) -> Json<serde_json::Value> {
    let worker_id = id.0;

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.tasks_for_worker(&worker_id)
    })
    .await;

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
        })),
    }
}

/// POST /workers/heartbeat - Worker heartbeat (public).
pub async fn worker_heartbeat_public(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let worker_id = body["worker_id"].as_str().unwrap_or("").to_string();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.worker_heartbeat(&worker_id)
    })
    .await;

    match result {
        Ok(Ok(())) => Json(json!({ "status": "ok" })),
        Ok(Err(e)) => Json(json!({ "status": "error", "message": e.to_string() })),
        Err(_) => Json(json!({ "status": "error", "message": "internal error" })),
    }
}
