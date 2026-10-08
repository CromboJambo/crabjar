//! Hermes kanban board source for habitat contract.
//!
//! Reads from the active Hermes kanban board SQLite database and projects
//! tasks into DAGR v3 format. This connects crabjar's habitat dashboard to
//! real multi-agent orchestration activity.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Error as SqlError};
use serde_json::Value;
use std::path::PathBuf;

/// A task from the Hermes kanban board.
#[derive(Debug, Clone)]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub assignee: Option<String>,
    pub priority: i64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub result: Option<String>,
    pub current_run_id: Option<i64>,
}

/// Board configuration for detecting stuck tasks.
#[derive(Debug, Clone)]
pub struct BoardConfig {
    pub slug: String,
    pub name: String,
    pub has_model: bool,
    pub model_name: Option<String>,
}

/// Read the active Hermes kanban board path.
///
/// Follows ~/.hermes/kanban/current symlink to find the active board database.
pub fn active_board_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| format!("HOME not set: {e}"))?;
    let kanban_dir = PathBuf::from(home).join(".hermes/kanban");

    // Read the "current" symlink to find active board
    let current_path = kanban_dir.join("current");
    let target = std::fs::read_link(&current_path)
        .map_err(|e| format!("failed to read kanban/current: {e}"))?;

    let db_path = target.join("kanban.db");
    if !db_path.exists() {
        return Err(format!("kanban.db not found at {:?}", db_path));
    }

    Ok(db_path)
}

/// List all kanban board database paths.
fn list_board_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var("HOME").map_err(|e| format!("HOME not set: {e}"))?;
    let boards_dir = PathBuf::from(home).join(".hermes/kanban/boards");

    if !boards_dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&boards_dir)
        .map_err(|e| format!("failed to list boards: {e}"))?
    {
        let entry = entry.map_err(|e| format!("failed to read board entry: {e}"))?;
        let db_path = entry.path().join("kanban.db");
        if db_path.exists() {
            paths.push(db_path);
        }
    }

    Ok(paths)
}

/// Read all non-archived tasks from all kanban boards.
pub fn read_all_tasks() -> Result<Vec<KanbanTask>, String> {
    let board_paths = list_board_paths()?;
    let mut all_tasks = Vec::new();

    for path in board_paths {
        match read_tasks(&path) {
            Ok(tasks) => all_tasks.extend(tasks),
            Err(e) => eprintln!("warning: failed to read board {:?}: {}", path, e),
        }
    }

    // Sort by created_at descending (most recent first)
    all_tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(all_tasks)
}

/// Read board configuration to check for model/provider settings.
pub fn read_board_config(board_path: &PathBuf) -> Option<BoardConfig> {
    let parent = board_path.parent()?;
    let config_path = parent.join("board.json");
    
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            // Check for model or provider configuration
            let has_model = content.contains("\"model\"") || content.contains("\"provider\"");
            
            let slug = board_path.file_name()?
                .to_string_lossy().into_owned();
            let name = parent.file_name()?
                .to_string_lossy().into_owned();
            
            Some(BoardConfig {
                slug,
                name,
                has_model,
                model_name: None,
            })
        }
        Err(_) => None,
    }
}

/// Detect boards with ready tasks but no model configured.
pub fn detect_stuck_boards(tasks: &[KanbanTask]) -> Vec<String> {
    let board_paths = list_board_paths().unwrap_or_default();
    let mut stuck = Vec::new();
    
    for path in &board_paths {
        // Check if this board has any ready tasks
        let has_ready_tasks = tasks.iter().any(|t| t.status == "ready");
        if !has_ready_tasks {
            continue;
        }
        
        // Check if board has model configured
        match read_board_config(path) {
            Some(config) => {
                if !config.has_model {
                    stuck.push(format!("board '{}' has ready tasks but no model/provider configured", config.name));
                }
            }
            None => {
                stuck.push(format!("board at {:?} has ready tasks but no board.json found", path));
            }
        }
    }
    
    stuck
}

/// Read all non-archived tasks from the kanban board.
pub fn read_tasks(board_path: &PathBuf) -> Result<Vec<KanbanTask>, String> {
    let conn = Connection::open(board_path)
        .map_err(|e| format!("failed to open kanban.db: {e}"))?;

    let mut stmt = conn.prepare(
        "SELECT id, title, status, assignee, priority, created_at, started_at, \
         completed_at, result, current_run_id \
         FROM tasks WHERE status != 'archived' ORDER BY created_at DESC"
    ).map_err(|e| format!("failed to prepare query: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(KanbanTask {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
            assignee: row.get(3)?,
            priority: row.get(4)?,
            created_at: row.get(5)?,
            started_at: row.get(6)?,
            completed_at: row.get(7)?,
            result: row.get(8)?,
            current_run_id: row.get(9)?,
        })
    }).map_err(|e| format!("failed to query tasks: {e}"))?;

    let mut tasks = Vec::new();
    for row in rows {
        match row {
            Ok(task) => tasks.push(task),
            Err(e) => eprintln!("warning: failed to read task row: {e}"),
        }
    }

    Ok(tasks)
}

/// Map a kanban status to a DAGR task state.
fn map_status(status: &str) -> &'static str {
    match status {
        "ready" | "todo" => "queued",
        "running" => "running",
        "blocked" => "blocked",
        "done" | "completed" => "done",
        "review" => "review",
        _ => "queued",
    }
}

/// Map a kanban status to a DAGR task kind.
fn map_kind(status: &str, priority: i64) -> &'static str {
    if priority >= 10 {
        "critical"
    } else if status == "running" {
        "active"
    } else {
        "task"
    }
}

/// Convert kanban tasks to DAGR v3 task objects.
pub fn to_dagr_tasks(tasks: &[KanbanTask]) -> Vec<Value> {
    let mut dagr_tasks = Vec::new();

    for t in tasks {
        let state = map_status(&t.status);
        let kind = map_kind(&t.status, t.priority);

        // Build attempts from run history if task has been running/completed
        let attempts = build_attempts(t);

        let owner = match &t.assignee {
            Some(a) if !a.is_empty() => a.clone(),
            _ => "maintainer".to_string(),
        };

        dagr_tasks.push(serde_json::json!({
            "id": format!("KANBAN-{}", t.id),
            "title": truncate_title(&t.title),
            "kind": kind,
            "owner": owner,
            "project": "hermes-kanban",
            "state": state,
            "deps": [],
            "priority": t.priority,
            "note": format!("hermes kanban task ({}). priority: {}", t.status, t.priority),
            "attempts": attempts,
        }));
    }

    dagr_tasks
}

/// Build DAGR attempt objects from kanban task run history.
fn build_attempts(task: &KanbanTask) -> Vec<Value> {
    let mut attempts = Vec::new();

    if task.current_run_id.is_some() || task.completed_at.is_some() {
        // Task has been executed at least once
        let started = task.started_at.unwrap_or(task.created_at);
        let ended = task.completed_at.unwrap_or(Utc::now().timestamp());

        let state = match task.status.as_str() {
            "done" | "completed" => "done",
            "running" => "running",
            _ => "failed",
        };

        attempts.push(serde_json::json!({
            "id": format!("{}-a1", task.id),
            "n": 1,
            "cause": { "type": "initial" },
            "actor": task.assignee.clone().unwrap_or_else(|| "agent".to_string()),
            "state": state,
            "started_at": chrono::DateTime::from_timestamp(started, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            "ended_at": if state == "running" { Value::Null } else {
                serde_json::Value::String(chrono::DateTime::from_timestamp(ended, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default())
            },
            "outcome": serde_json::json!({
                "result": state,
                "evidence": "reported"
            })
        }));
    }

    attempts
}

fn truncate_title(title: &str) -> String {
    let t = title.trim();
    if t.chars().count() > 48 {
        let truncated: String = t.chars().take(45).collect();
        format!("{truncated}…")
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ready_to_queued() {
        assert_eq!(map_status("ready"), "queued");
        assert_eq!(map_status("todo"), "queued");
    }

    #[test]
    fn maps_running_to_running() {
        assert_eq!(map_status("running"), "running");
    }

    #[test]
    fn maps_done_to_done() {
        assert_eq!(map_status("done"), "done");
        assert_eq!(map_status("completed"), "done");
    }

    #[test]
    fn maps_blocked_to_blocked() {
        assert_eq!(map_status("blocked"), "blocked");
    }

    #[test]
    fn high_priority_is_critical_kind() {
        assert_eq!(map_kind("ready", 10), "critical");
        assert_eq!(map_kind("ready", 15), "critical");
    }

    #[test]
    fn normal_priority_is_task_kind() {
        assert_eq!(map_kind("ready", 5), "task");
    }
}