//! Worker-related API handlers.

use crate::api::ConductorState;
use crate::task_spec::TaskSpec;
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

/// POST /workers/deploy - Deploy a worker sandbox for a task (public).
///
/// Accepts a `TaskSpec` and `supervisor_id`, creates a deployment record,
/// and returns the deployment ID. Part of SUPERVISOR-DEPLOYMENT-READINESS Phase 1.
pub async fn deploy_worker(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let supervisor_id = body["supervisor_id"].as_str().unwrap_or("").to_string();

    // Parse and validate the task spec
    let task_spec: TaskSpec = match serde_json::from_value(body["task"].clone()) {
        Ok(spec) => spec,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("invalid task spec: {}", e)
            }));
        }
    };

    // Validate required fields
    if let Err(e) = task_spec.validate() {
        return Json(json!({
            "status": "error",
            "message": e
        }));
    }

    // Validate repo
    if let Err(e) = task_spec.validate_repo() {
        return Json(json!({
            "status": "error",
            "message": e
        }));
    }

    info!(
        supervisor_id = %supervisor_id,
        task_id = %task_spec.id,
        repo = %task_spec.repo,
        "Worker deployment requested"
    );

    let task_id = task_spec.id.clone();
    let repo = task_spec.repo.clone();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        let store = c.store();

        // Check supervisor readiness if supervisor_id provided
        if !supervisor_id.is_empty() {
            match store.get_supervisor_status(&supervisor_id) {
                Ok(Some(status)) => {
                    if !status.ready {
                        return Err(anyhow::anyhow!("supervisor '{supervisor_id}' is not ready"));
                    }
                }
                Ok(None) => {
                    // Supervisor not registered yet — allow but warn
                    tracing::warn!(
                        supervisor_id = %supervisor_id,
                        "Supervisor not found in status table, proceeding anyway"
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to check supervisor status");
                }
            }
        }

        // Create deployment record (store handles ID generation and timestamp)
        let task_spec_value = serde_json::to_value(&task_spec)
            .map_err(|e| anyhow::anyhow!("failed to serialize task spec: {e}"))?;

        let deployment = store.create_deployment(supervisor_id, task_spec_value)?;
        Ok(deployment)
    })
    .await;

    match result {
        Ok(Ok(deployment)) => Json(json!({
            "status": "deployed",
            "deployment_id": deployment.id,
            "supervisor_id": deployment.supervisor_id,
            "task_id": task_id,
            "repo": repo,
            "initial_status": "pending"
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

/// GET /deployments - List all deployments (protected).
pub async fn list_deployments(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.store().list_deployments(None, None)
    })
    .await;

    match result {
        Ok(Ok(deployments)) => Json(json!({
            "deployments": deployments.iter().map(|d| json!({
                "id": d.id,
                "supervisor_id": d.supervisor_id,
                "status": d.status.as_str(),
                "created_at": d.created_at
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

/// GET /deployments/{id} - Get deployment status (protected).
pub async fn get_deployment(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    id: Path<String>,
) -> Json<serde_json::Value> {
    let deployment_id = id.0;
    let lookup_id = deployment_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.store().get_deployment(&lookup_id)
    })
    .await;

    match result {
        Ok(Ok(Some(deployment))) => Json(json!({
            "id": deployment.id,
            "supervisor_id": deployment.supervisor_id,
            "task_spec": deployment.task_spec,
            "sandbox_id": deployment.sandbox_id,
            "status": deployment.status.as_str(),
            "outcome": deployment.outcome,
            "created_at": deployment.created_at,
            "completed_at": deployment.completed_at,
            "error": deployment.error
        })),
        Ok(Ok(None)) => Json(json!({
            "status": "not_found",
            "deployment_id": deployment_id
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

/// POST /workers/heartbeat/supervisor - Supervisor heartbeat with health status (public).
pub async fn supervisor_heartbeat(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = body["id"].as_str().unwrap_or("").to_string();
    let ready = body["ready"].as_bool().unwrap_or(false);
    let health = body.get("health").cloned().unwrap_or(serde_json::json!({}));

    if id.is_empty() {
        return Json(json!({
            "status": "error",
            "message": "supervisor id is required"
        }));
    }

    info!(supervisor_id = %id, ready = ready, "Supervisor heartbeat");

    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.store().record_heartbeat(&id, ready, &health)
    })
    .await;

    match result {
        Ok(Ok(())) => Json(json!({ "status": "ok" })),
        Ok(Err(e)) => Json(json!({ "status": "error", "message": e.to_string() })),
        Err(_) => Json(json!({ "status": "error", "message": "internal error" })),
    }
}

/// GET /supervisors/status - List all supervisor statuses (protected).
pub async fn list_supervisor_statuses(
    axum::extract::State(conductor): axum::extract::State<ConductorState>,
) -> Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(move || {
        let c = conductor.blocking_lock();
        c.store().list_supervisor_statuses()
    })
    .await;

    match result {
        Ok(Ok(statuses)) => Json(json!({
            "supervisors": statuses.iter().map(|s| json!({
                "id": s.id,
                "ready": s.ready,
                "last_heartbeat": s.last_heartbeat,
                "active_deployments": s.active_deployments,
                "health": s.health
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
