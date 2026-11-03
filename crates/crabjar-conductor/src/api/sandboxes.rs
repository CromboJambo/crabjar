//! Sandbox management API endpoints.

use crate::api::ConductorState;
use crate::sandbox::SandboxConfig;
use axum::{Json, extract::Path};
use serde_json::json;

/// Create a new sandbox container.
pub async fn create_sandbox_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    body: Json<SandboxConfig>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.sandbox_manager().create_sandbox(body.0)
    })
    .await;

    match result {
        Ok(Ok(state)) => Json(json!({
            "success": true,
            "sandbox_id": state.container_id,
            "container_id": state.container_id,
            "created_at": state.created_at
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Sandbox creation failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// List all sandbox containers.
pub async fn list_sandboxes_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.sandbox_manager().list_sandboxes()
    })
    .await;

    match result {
        Ok(sandboxes) => Json(json!({
            "success": true,
            "count": sandboxes.len(),
            "sandboxes": sandboxes.iter().map(|s| json!({
                "sandbox_id": s.container_id,
                "image": s.config.image,
                "network_mode": s.config.network_mode,
                "worker_id": s.worker_id,
                "created_at": s.created_at
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Get sandbox by ID.
pub async fn get_sandbox_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: Path<String>,
) -> Json<serde_json::Value> {
    let sandbox_id = id.0.clone();
    let sandbox_id_for_closure = sandbox_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.sandbox_manager().get_sandbox(&sandbox_id_for_closure)
    })
    .await;

    match result {
        Ok(Some(state)) => Json(json!({
            "success": true,
            "sandbox_id": state.container_id,
            "container_id": state.container_id,
            "image": state.config.image,
            "network_mode": state.config.network_mode,
            "worker_id": state.worker_id,
            "created_at": state.created_at
        })),
        Ok(None) => Json(json!({
            "success": false,
            "error": format!("Sandbox {} not found", sandbox_id)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Destroy a sandbox by ID.
pub async fn destroy_sandbox_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: Path<String>,
) -> Json<serde_json::Value> {
    let sandbox_id = id.0.clone();
    let sandbox_id_for_closure = sandbox_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.sandbox_manager().destroy_sandbox(&sandbox_id_for_closure)
    })
    .await;

    match result {
        Ok(Ok(())) => Json(json!({
            "success": true,
            "sandbox_id": sandbox_id,
            "message": "Sandbox destroyed"
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Sandbox destruction failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}

/// Assign a worker to a sandbox.
pub async fn assign_worker_to_sandbox_handler(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: Path<String>,
    body: Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let worker_id = match body["worker_id"].as_str() {
        Some(wid) => wid.to_string(),
        None => return Json(json!({
            "success": false,
            "error": "Missing worker_id in request body"
        })),
    };

    let sandbox_id = id.0.clone();
    let sandbox_id_for_closure = sandbox_id.clone();
    let worker_id_for_closure = worker_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.sandbox_manager().assign_worker(&sandbox_id_for_closure, &worker_id_for_closure)
    })
    .await;

    match result {
        Ok(Ok(())) => Json(json!({
            "success": true,
            "sandbox_id": sandbox_id,
            "worker_id": worker_id,
            "message": "Worker assigned to sandbox"
        })),
        Ok(Err(e)) => Json(json!({
            "success": false,
            "error": format!("Worker assignment failed: {}", e)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": format!("Task join error: {}", e)
        })),
    }
}
