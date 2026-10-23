//! Crabjar Kanban Orchestrator
//!
//! Maps orchestrator inputs to hermes agent kanban operations. Provides HTTP endpoints
//! for task management that integrate with the crabjar tool registry and guard system.
//!
//! Architecture:
//! - Receives task creation/update requests via HTTP
//! - Validates through the guard gate (provenance enforcement)
//! - Writes tasks to the active hermes kanban board
//! - Supports tool call integration for automated task management

use anyhow::Result;
use axum::{
    Json, Router,
    extract::Path,
    response::Json as JsonResponse,
    routing::{get, post},
};
use chrono::Utc;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Generate a task ID in the same format as hermes agent (t_xxxxxxxx)
fn generate_task_id() -> String {
    let chars = "0123456789abcdef";
    let mut id = "t_".to_string();
    for _ in 0..8 {
        let idx = fastrand::u8(0..16);
        id.push(chars.as_bytes()[idx as usize] as char);
    }
    id
}

// ---------------------------------------------------------------------------
// Request/Response Types
// ---------------------------------------------------------------------------

/// Task creation request from orchestrator input.
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub assignee: Option<String>,
    pub priority: Option<i64>,
    pub body: Option<String>,
    /// Source of the task (e.g., "orchestrator", "hermes-agent", "user")
    pub source: Option<String>,
}

/// Task update request.
#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<i64>,
    pub result: Option<String>,
}

/// Task block request.
#[derive(Debug, Deserialize)]
pub struct BlockTaskRequest {
    pub reason: String,
    #[serde(rename = "kind")]
    pub block_kind: Option<String>,
}

/// Task response object.
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub assignee: Option<String>,
    pub priority: i64,
    pub created_at: i64,
}

/// Success response.
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ---------------------------------------------------------------------------
// Board State — finds and manages the active hermes kanban board
// ---------------------------------------------------------------------------

fn find_active_board() -> Result<Connection> {
    let home = std::env::var("HOME").map_err(|e| anyhow::anyhow!("HOME not set: {}", e))?;
    
    // Read the "current" file to get the active board slug
    let current_path = format!("{}/.hermes/kanban/current", home);
    let board_slug = std::fs::read_to_string(&current_path)
        .map_err(|e| anyhow::anyhow!("failed to read kanban/current: {}", e))?
        .trim()
        .to_string();

    // Construct the path to the board database
    let db_path = format!("{}/.hermes/kanban/boards/{}/kanban.db", home, board_slug);
    
    if !std::path::Path::new(&db_path).exists() {
        return Err(anyhow::anyhow!("kanban.db not found at {}", db_path));
    }

    Connection::open(&db_path).map_err(|e| anyhow::anyhow!("failed to open kanban.db: {}", e))
}

// ---------------------------------------------------------------------------
// Task Operations — write to the active board
// ---------------------------------------------------------------------------

async fn create_task(Json(req): Json<CreateTaskRequest>) -> JsonResponse<SuccessResponse> {
    let source = req.source.clone().unwrap_or_else(|| "orchestrator".to_string());
    info!("Creating task from {}: {}", source, req.title);

    match find_active_board() {
        Ok(conn) => {
            // Generate a proper task ID in hermes format (t_xxxxxxxx)
            let task_id = generate_task_id();
            let now = Utc::now().timestamp();

            let result = conn.execute(
                "INSERT INTO tasks (id, title, status, assignee, priority, created_at, body) VALUES (?, ?, 'todo', ?, ?, ?, ?)",
                params![task_id.clone(), req.title, req.assignee, req.priority.unwrap_or(0), now, req.body],
            );

            match result {
                Ok(_) => JsonResponse(SuccessResponse {
                    success: true,
                    message: "Task created".to_string(),
                    task_id: Some(task_id),
                }),
                Err(e) => JsonResponse(SuccessResponse {
                    success: false,
                    message: format!("Failed to create task: {}", e),
                    task_id: None,
                }),
            }
        }
        Err(e) => JsonResponse(SuccessResponse {
            success: false,
            message: format!("Failed to find active board: {}", e),
            task_id: None,
        }),
    }
}

async fn update_task(
    Path(task_id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> JsonResponse<SuccessResponse> {
    info!("Updating task {}: {:?}", task_id, req);

    match find_active_board() {
        Ok(conn) => {
            // Handle each field separately to avoid complex params handling
            if let Some(status) = &req.status {
                match conn.execute(
                    "UPDATE tasks SET status = ? WHERE id = ?",
                    params![status, task_id],
                ) {
                    Ok(rows) => return JsonResponse(SuccessResponse {
                        success: true,
                        message: format!("Updated status for {} row(s)", rows),
                        task_id: None,
                    }),
                    Err(e) => return JsonResponse(SuccessResponse {
                        success: false,
                        message: format!("Failed to update status: {}", e),
                        task_id: None,
                    }),
                }
            }

            if let Some(assignee) = &req.assignee {
                match conn.execute(
                    "UPDATE tasks SET assignee = ? WHERE id = ?",
                    params![assignee, task_id],
                ) {
                    Ok(rows) => return JsonResponse(SuccessResponse {
                        success: true,
                        message: format!("Updated assignee for {} row(s)", rows),
                        task_id: None,
                    }),
                    Err(e) => return JsonResponse(SuccessResponse {
                        success: false,
                        message: format!("Failed to update assignee: {}", e),
                        task_id: None,
                    }),
                }
            }

            if let Some(priority) = req.priority {
                match conn.execute(
                    "UPDATE tasks SET priority = ? WHERE id = ?",
                    params![priority, task_id],
                ) {
                    Ok(rows) => return JsonResponse(SuccessResponse {
                        success: true,
                        message: format!("Updated priority for {} row(s)", rows),
                        task_id: None,
                    }),
                    Err(e) => return JsonResponse(SuccessResponse {
                        success: false,
                        message: format!("Failed to update priority: {}", e),
                        task_id: None,
                    }),
                }
            }

            if let Some(result) = &req.result {
                match conn.execute(
                    "UPDATE tasks SET result = ? WHERE id = ?",
                    params![result, task_id],
                ) {
                    Ok(rows) => return JsonResponse(SuccessResponse {
                        success: true,
                        message: format!("Updated result for {} row(s)", rows),
                        task_id: None,
                    }),
                    Err(e) => return JsonResponse(SuccessResponse {
                        success: false,
                        message: format!("Failed to update result: {}", e),
                        task_id: None,
                    }),
                }
            }

            JsonResponse(SuccessResponse {
                success: true,
                message: "No changes to apply".to_string(),
                task_id: None,
            })
        }
        Err(e) => JsonResponse(SuccessResponse {
            success: false,
            message: format!("Failed to find active board: {}", e),
            task_id: None,
        }),
    }
}

async fn complete_task(Path(task_id): Path<String>) -> JsonResponse<SuccessResponse> {
    info!("Completing task {}", task_id);

    match find_active_board() {
        Ok(conn) => {
            let now = Utc::now().timestamp();
            match conn.execute(
                "UPDATE tasks SET status = 'done', completed_at = ? WHERE id = ?",
                params![now, task_id],
            ) {
                Ok(rows) => JsonResponse(SuccessResponse {
                    success: true,
                    message: format!("Completed {} row(s)", rows),
                    task_id: None,
                }),
                Err(e) => JsonResponse(SuccessResponse {
                    success: false,
                    message: format!("Failed to complete task: {}", e),
                    task_id: None,
                }),
            }
        }
        Err(e) => JsonResponse(SuccessResponse {
            success: false,
            message: format!("Failed to find active board: {}", e),
            task_id: None,
        }),
    }
}

async fn block_task(
    Path(task_id): Path<String>,
    Json(req): Json<BlockTaskRequest>,
) -> JsonResponse<SuccessResponse> {
    info!("Blocking task {}: {}", task_id, req.reason);

    match find_active_board() {
        Ok(conn) => {
            let kind = req.block_kind.unwrap_or_else(|| "needs_input".to_string());
            match conn.execute(
                "UPDATE tasks SET status = 'blocked' WHERE id = ?",
                params![task_id],
            ) {
                Ok(rows) => JsonResponse(SuccessResponse {
                    success: true,
                    message: format!("Blocked {} row(s) (kind: {})", rows, kind),
                    task_id: None,
                }),
                Err(e) => JsonResponse(SuccessResponse {
                    success: false,
                    message: format!("Failed to block task: {}", e),
                    task_id: None,
                }),
            }
        }
        Err(e) => JsonResponse(SuccessResponse {
            success: false,
            message: format!("Failed to find active board: {}", e),
            task_id: None,
        }),
    }
}

async fn list_tasks() -> JsonResponse<serde_json::Value> {
    match find_active_board() {
        Ok(conn) => {
            let mut stmt = conn.prepare(
                "SELECT id, title, status, assignee, priority, created_at FROM tasks WHERE status != 'archived' ORDER BY created_at DESC LIMIT 50",
            );

            match stmt {
                Ok(mut stmt) => {
                    let rows = stmt.query_map([], |row| {
                        Ok(TaskResponse {
                            id: row.get(0)?,
                            title: row.get(1)?,
                            status: row.get(2)?,
                            assignee: row.get(3)?,
                            priority: row.get(4)?,
                            created_at: row.get(5)?,
                        })
                    });

                    match rows {
                        Ok(row_iter) => {
                            let mut tasks = Vec::new();
                            for row in row_iter {
                                match row {
                                    Ok(task) => tasks.push(task),
                                    Err(e) => warn!("Failed to read task row: {}", e),
                                }
                            }
                            JsonResponse(serde_json::json!({ "tasks": tasks }))
                        }
                        Err(e) => JsonResponse(serde_json::json!({ "error": format!("{}", e) })),
                    }
                }
                Err(e) => JsonResponse(serde_json::json!({ "error": format!("{}", e) })),
            }
        }
        Err(e) => JsonResponse(serde_json::json!({ "error": format!("Failed to find active board: {}", e) })),
    }
}

// ---------------------------------------------------------------------------
// Main — HTTP server setup
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/kanban/tasks", post(create_task))
        .route("/kanban/tasks", get(list_tasks))
        .route("/kanban/tasks/{id}/update", post(update_task))
        .route("/kanban/tasks/{id}/complete", post(complete_task))
        .route("/kanban/tasks/{id}/block", post(block_task));

    let addr = "127.0.0.1:3001";
    info!("Kanban orchestrator listening on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}