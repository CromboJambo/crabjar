use crate::commands::goals::submit_goal_to_conductor;

/// Handle fleet commands (kanban → conductor bridge)
pub async fn handle_fleet_submit_kanban_task(
    task_id: &str,
    conductor_url: &str,
    token: &str,
) -> serde_json::Value {
    // Read the kanban task details from Hermes board
    let all_tasks = match crabjar_lib::kanban_source::read_all_tasks() {
        Ok(tasks) => tasks,
        Err(e) => {
            return error_response(&format!("failed to read kanban tasks: {}", e), true);
        }
    };

    // Find the specific task by ID
    let task = all_tasks.iter().find(|t| t.id == task_id);

    match task {
        None => {
            return error_response(&format!("kanban task '{}' not found", task_id), true);
        }
        Some(task) => {
            // Build goal description from task details
            let status_info = format!("status: {}, priority: {}", task.status, task.priority);

            let assignee_info = match &task.assignee {
                Some(a) if !a.is_empty() => format!("assignee: {}", a),
                _ => String::new(),
            };

            let suffix = if assignee_info.is_empty() {
                ""
            } else {
                &format!(" {}", assignee_info)
            };
            let goal_description = format!(
                "Hermes kanban task '{}' ({}){}",
                task.title, status_info, suffix
            );

            // Submit to conductor API via HTTP POST
            let url = format!("{}/goals", conductor_url);

            let body = serde_json::json!({
                "description": goal_description,
                "metadata": {
                    "kanban_task_id": task.id,
                    "original_title": task.title,
                    "status": task.status,
                    "priority": task.priority,
                    "assignee": task.assignee.clone()
                }
            });

            // Build HTTP request using std net/http via curl or reqwest
            let result = submit_goal_to_conductor(&url, &body, token).await;

            match result {
                Ok(response) => response,
                Err(e) => error_response(&format!("failed to submit goal: {}", e), true),
            }
        }
    }
}

fn error_response(message: &str, show_usage: bool) -> serde_json::Value {
    let mut response = json!({
        "success": false,
        "error": message,
    });

    if show_usage {
        response["usage"] = json!([]);
    }

    response
}

use serde_json::json;
