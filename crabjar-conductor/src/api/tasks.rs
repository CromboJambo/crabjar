//! Task management routes: fetch tasks for workers and report results.

use crate::api::ConductorState;
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;

/// GET /tasks/for-worker/{worker_id} - Fetch pending tasks for a specific worker.
pub async fn tasks_for_worker(
    State(conductor): State<ConductorState>,
    Path(worker_id): Path<String>,
) -> Json<serde_json::Value> {
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

/// POST /tasks/{id}/result - Report the result of a completed task.
pub async fn report_task_result(
    State(conductor): State<ConductorState>,
    Path(task_id): Path<String>,
    body: Json<serde_json::Value>,
) -> Json<serde_json::Value> {
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
    })
    .await;

    match result {
        Ok(Ok(Some(new_token))) => Json(json!({
            "status": "recorded",
            "trust_level_changed": true,
            "auth_token": new_token
        })),
        Ok(Ok(None)) => Json(json!({ "status": "recorded" })),
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
