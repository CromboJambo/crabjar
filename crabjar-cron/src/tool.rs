//! Hermes-compatible `cronjob_manage` tool implementation.

use anyhow::Result;
use serde_json::{json, Value};

use crate::store::CronStore;
use crate::types::{CreateJobRequest, UpdateJobRequest};

/// Handle a `cronjob_manage` tool invocation.
pub async fn handle_cronjob_manage(
    store: &CronStore,
    params: &Value,
) -> Result<Value> {
    let action = params["action"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "create" => handle_create(store, params).await,
        "list" => handle_list(store),
        "update" => handle_update(store, params).await,
        "pause" => handle_pause(store, params).await,
        "resume" => handle_resume(store, params).await,
        "remove" => handle_remove(store, params).await,
        "run" => handle_run(store, params).await,
        _ => Err(anyhow::anyhow!("Unknown action: {action}")),
    }
}

async fn handle_create(store: &CronStore, params: &Value) -> Result<Value> {
    let schedule = params["schedule"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing required field: schedule"))?;

    let prompt = params["prompt"].as_str().map(String::from);
    let name = params["name"].as_str().map(String::from);

    let skills: Vec<String> = if let Some(arr) = params["skills"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    } else {
        Vec::new()
    };

    let script = params["script"].as_str().map(String::from);
    let deliver = params["deliver"].as_str().map(String::from);
    let no_agent = params["no_agent"].as_bool().unwrap_or(false);
    let paused = params["paused"].as_bool().unwrap_or(false);
    let paused_reason = params["paused_reason"].as_str().map(String::from);

    let req = CreateJobRequest {
        schedule: schedule.to_string(),
        prompt,
        name,
        skills,
        script,
        deliver,
        no_agent,
        paused,
        paused_reason,
    };

    let job = store.create_job(&req)?;

    if paused {
        store.pause_job(&job.id, req.paused_reason.as_deref())?;
    }

    Ok(json!({
        "success": true,
        "job_id": job.id,
        "name": job.name,
        "schedule": job.schedule,
        "message": format!("Cron job created: {}", job.id)
    }))
}

fn handle_list(store: &CronStore) -> Result<Value> {
    let jobs = store.list_jobs()?;

    let job_values: Vec<Value> = jobs.iter().map(|job| {
        json!({
            "id": job.id,
            "name": job.name,
            "schedule": job.schedule,
            "status": job.status.to_string(),
            "last_run_at": job.last_run_at.map(|t| t.to_rfc3339()),
            "next_run_at": job.next_run_at.map(|t| t.to_rfc3339()),
            "failure_count": job.failure_count,
            "no_agent": job.no_agent
        })
    }).collect();

    Ok(json!({
        "success": true,
        "jobs": job_values,
        "count": job_values.len()
    }))
}

async fn handle_update(store: &CronStore, params: &Value) -> Result<Value> {
    let id = get_job_id(params)?;

    let req = UpdateJobRequest {
        schedule: params["schedule"].as_str().map(String::from),
        prompt: params["prompt"].as_str().map(String::from),
        name: params["name"].as_str().map(String::from),
        skills: params["skills"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        }),
        script: params["script"].as_str().map(String::from),
        deliver: params["deliver"].as_str().map(String::from),
        no_agent: params["no_agent"].as_bool(),
    };

    let job = store.update_job(&id, &req)?;

    Ok(json!({
        "success": true,
        "job_id": job.id,
        "message": format!("Job updated: {}", id)
    }))
}

async fn handle_pause(store: &CronStore, params: &Value) -> Result<Value> {
    let id = get_job_id(params)?;
    let reason = params["reason"].as_str().map(String::from);

    store.pause_job(&id, reason.as_deref())?;

    Ok(json!({
        "success": true,
        "job_id": id,
        "message": format!("Job paused: {}", id)
    }))
}

async fn handle_resume(store: &CronStore, params: &Value) -> Result<Value> {
    let id = get_job_id(params)?;

    store.resume_job(&id)?;

    Ok(json!({
        "success": true,
        "job_id": id,
        "message": format!("Job resumed: {}", id)
    }))
}

async fn handle_remove(store: &CronStore, params: &Value) -> Result<Value> {
    let id = get_job_id(params)?;

    store.remove_job(&id)?;

    Ok(json!({
        "success": true,
        "job_id": id,
        "message": format!("Job removed: {}", id)
    }))
}

async fn handle_run(store: &CronStore, params: &Value) -> Result<Value> {
    let id = get_job_id(params)?;

    // Mark as running and submit to conductor
    store.mark_running(&id)?;

    Ok(json!({
        "success": true,
        "job_id": id,
        "message": format!("Job triggered: {}", id)
    }))
}

fn get_job_id(params: &Value) -> Result<String> {
    params["job_id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("Missing required field: job_id"))
}