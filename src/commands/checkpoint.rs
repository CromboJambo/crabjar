use crabjar_durable_objects::{CheckpointConfig, CheckpointStore};
use crabjar_lib::CheckpointCommand;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn get_checkpoint_db_path() -> PathBuf {
    let home = dirs::home_dir().expect("could not determine home directory");
    home.join(".crabjar").join("checkpoints.db")
}

pub fn handle_checkpoint_command(command: CheckpointCommand) -> Result<serde_json::Value, String> {
    match command {
        CheckpointCommand::Create {
            task_id,
            state_file,
            message,
        } => create_checkpoint(&task_id, state_file.as_deref(), message.as_deref()),
        CheckpointCommand::List { task_id, limit } => list_checkpoints(&task_id, limit),
        CheckpointCommand::Latest { task_id } => get_latest_checkpoint(&task_id),
        CheckpointCommand::Show { id } => show_checkpoint(&id),
        CheckpointCommand::Prune {
            task_id,
            keep_count,
            ..
        } => prune_checkpoints(task_id.as_deref(), keep_count),
    }
}

fn create_checkpoint(
    task_id: &str,
    state_file: Option<&str>,
    message: Option<&str>,
) -> Result<serde_json::Value, String> {
    let db_path = get_checkpoint_db_path();
    let config = CheckpointConfig::new(db_path.to_string_lossy().to_string());
    let store = CheckpointStore::open(&config.db_path).map_err(|e| e.to_string())?;

    // Read state from file if provided, otherwise use a default payload
    let payload = if let Some(path) = state_file {
        let content =
            fs::read_to_string(path).map_err(|e| format!("failed to read state file: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("invalid JSON in state file: {}", e))?
    } else {
        json!({ "timestamp": chrono::Utc::now().to_rfc3339() })
    };

    // Store payload as string for now (payload_path would be used for large payloads)
    let id = store
        .create_checkpoint(task_id, payload, None)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "checkpoint_id": id,
        "task_id": task_id,
        "message": message
    }))
}

fn list_checkpoints(task_id: &str, limit: usize) -> Result<serde_json::Value, String> {
    let db_path = get_checkpoint_db_path();
    let config = CheckpointConfig::new(db_path.to_string_lossy().to_string());
    let store = CheckpointStore::open(&config.db_path).map_err(|e| e.to_string())?;

    let checkpoints = store.list_checkpoints(task_id).map_err(|e| e.to_string())?;
    let recent: Vec<_> = checkpoints.into_iter().take(limit).collect();

    Ok(json!({
        "success": true,
        "task_id": task_id,
        "count": recent.len(),
        "checkpoints": recent
    }))
}

fn get_latest_checkpoint(task_id: &str) -> Result<serde_json::Value, String> {
    let db_path = get_checkpoint_db_path();
    let config = CheckpointConfig::new(db_path.to_string_lossy().to_string());
    let store = CheckpointStore::open(&config.db_path).map_err(|e| e.to_string())?;

    match store
        .get_latest_checkpoint(task_id)
        .map_err(|e| e.to_string())?
    {
        Some(cp) => Ok(json!({
            "success": true,
            "checkpoint": cp
        })),
        None => Ok(json!({
            "success": true,
            "checkpoint": null,
            "message": "no checkpoints found for task"
        })),
    }
}

fn show_checkpoint(id: &str) -> Result<serde_json::Value, String> {
    let db_path = get_checkpoint_db_path();
    let config = CheckpointConfig::new(db_path.to_string_lossy().to_string());
    let store = CheckpointStore::open(&config.db_path).map_err(|e| e.to_string())?;

    match store.get_checkpoint(id).map_err(|e| e.to_string())? {
        Some(cp) => Ok(json!({
            "success": true,
            "checkpoint": cp
        })),
        None => Err(format!("checkpoint {} not found", id)),
    }
}

fn prune_checkpoints(
    task_id: Option<&str>,
    keep_count: Option<usize>,
) -> Result<serde_json::Value, String> {
    let db_path = get_checkpoint_db_path();
    let base_config = CheckpointConfig::new(db_path.to_string_lossy().to_string());

    // If keep_count is specified, create a config with that retention count
    let config = if let Some(keep) = keep_count {
        CheckpointConfig {
            db_path: base_config.db_path.clone(),
            retention_count: keep as u32,
            ..base_config
        }
    } else {
        base_config
    };

    // Use new() with config to respect custom retention settings
    let store = CheckpointStore::new(&config).map_err(|e| e.to_string())?;

    if let Some(tid) = task_id {
        // Prune for specific task
        let pruned = store
            .prune_old_checkpoints(tid)
            .map_err(|e| e.to_string())?;
        Ok(json!({
            "success": true,
            "pruned_count": pruned,
            "retention_count": config.retention_count,
            "task_id": tid
        }))
    } else {
        // Prune for all tasks — use a wildcard approach
        // Since the store API requires a task_id, we'll prune with an empty string
        // which should match nothing, so this is effectively a no-op without task_id
        Ok(json!({
            "success": true,
            "pruned_count": 0,
            "retention_count": config.retention_count,
            "message": "specify task_id to prune checkpoints"
        }))
    }
}
