//! Cell subcommand handlers — durable session state via celld cells.

use crabjar_lib::{celld_session, CellCommand, CellWorkloadCommand};
use serde_json::json;

pub fn handle_cell_command(command: CellCommand) -> serde_json::Value {
    match command {
        CellCommand::Put {
            session,
            key,
            value,
            r#type,
        } => handle_put(&session, &key, &value, &r#type),
        CellCommand::Get { session, key } => handle_get(&session, &key),
        CellCommand::List { session } => handle_list(&session),
        CellCommand::Delete { session, key } => handle_delete(&session, &key),
        CellCommand::Workload { command } => handle_workload_command(command),
    }
}

fn handle_put(
    session: &str,
    key: &str,
    value_str: &str,
    entry_type: &str,
) -> serde_json::Value {
    let value: serde_json::Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            return json!({
                "success": false,
                "error": format!("invalid JSON value: {}", e),
                "usage": ["crabjar cell put <session> <key> '<json>' [--type <t>]"]
            });
        }
    };

    let client = celld_session::SessionClient::new(session);
    match client.put(key, &value, entry_type) {
        Ok(meta) => json!({
            "success": true,
            "session": session,
            "key": key,
            "meta": meta
        }),
        Err(e) => json!({
            "success": false,
            "error": e,
            "usage": ["crabjar cell put <session> <key> '<json>' [--type <t>]"]
        }),
    }
}

fn handle_get(session: &str, key: &str) -> serde_json::Value {
    let client = celld_session::SessionClient::new(session);
    match client.get(key) {
        Ok(Some(value)) => json!({
            "success": true,
            "session": session,
            "key": key,
            "value": value
        }),
        Ok(None) => json!({
            "success": false,
            "error": "not_found",
            "session": session,
            "key": key
        }),
        Err(e) => json!({
            "success": false,
            "error": e,
            "usage": ["crabjar cell get <session> <key>"]
        }),
    }
}

fn handle_list(session: &str) -> serde_json::Value {
    let client = celld_session::SessionClient::new(session);
    match client.list() {
        Ok(entries) => json!({
            "success": true,
            "session": session,
            "count": entries.len(),
            "entries": entries
        }),
        Err(e) => json!({
            "success": false,
            "error": e,
            "usage": ["crabjar cell list <session>"]
        }),
    }
}

fn handle_delete(session: &str, key: &str) -> serde_json::Value {
    let client = celld_session::SessionClient::new(session);
    match client.delete(key) {
        Ok(()) => json!({
            "success": true,
            "session": session,
            "key": key,
            "deleted": true
        }),
        Err(e) => json!({
            "success": false,
            "error": e,
            "usage": ["crabjar cell delete <session> <key>"]
        }),
    }
}

fn handle_workload_command(command: CellWorkloadCommand) -> serde_json::Value {
    match command {
        CellWorkloadCommand::Create { session, name, steps } => {
            if steps.is_empty() {
                return json!({
                    "success": false,
                    "error": "at least one step required",
                    "usage": ["crabjar cell workload create <session> <name> <step1> [step2] ..."]
                });
            }

            let client = celld_session::SessionClient::new(&session);
            let workload = celld_session::CheckpointedWorkload::new(&name, steps);
            let key = format!("workload:{}", name);

            match client.put(&key, &serde_json::to_value(&workload).unwrap(), "workload") {
                Ok(meta) => json!({
                    "success": true,
                    "session": session,
                    "name": name,
                    "steps_created": workload.steps.len(),
                    "meta": meta
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e,
                    "usage": ["crabjar cell workload create <session> <name> <step1> [step2] ..."]
                }),
            }
        }

        CellWorkloadCommand::Status { session, name } => {
            let client = celld_session::SessionClient::new(&session);
            let key = format!("workload:{}", name);

            match client.get(&key) {
                Ok(Some(value)) => {
                    let workload: celld_session::CheckpointedWorkload =
                        match serde_json::from_value(value) {
                            Ok(w) => w,
                            Err(e) => {
                                return json!({ "success": false, "error": format!("invalid workload data: {}", e) })
                            }
                        };
                    json!({
                        "success": true,
                        "session": session,
                        "name": name,
                        "progress": workload.progress_summary(),
                        "complete": workload.is_complete(),
                        "next_step": workload.next_pending().map(|i| &workload.steps[i].id),
                        "steps": workload.steps
                    })
                }
                Ok(None) => json!({
                    "success": false,
                    "error": "workload not found",
                    "session": session,
                    "name": name
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e,
                    "usage": ["crabjar cell workload status <session> <name>"]
                }),
            }
        }

        CellWorkloadCommand::Step { session, name, result } => {
            let client = celld_session::SessionClient::new(&session);
            let key = format!("workload:{}", name);

            let value = match client.get(&key) {
                Ok(Some(v)) => v,
                Ok(None) => {
                    return json!({ "success": false, "error": "workload not found", "session": session, "name": name })
                }
                Err(e) => {
                    return json!({ "success": false, "error": e })
                }
            };

            let mut workload: celld_session::CheckpointedWorkload =
                match serde_json::from_value(value) {
                    Ok(w) => w,
                    Err(e) => {
                        return json!({ "success": false, "error": format!("invalid workload data: {}", e) })
                    }
                };

            let next_idx = match workload.next_pending() {
                Some(i) => i,
                None => {
                    return json!({
                        "success": true,
                        "session": session,
                        "name": name,
                        "complete": true,
                        "message": "all steps already complete"
                    })
                }
            };

            // Mark as in-progress, execute (simulated), mark complete
            workload.steps[next_idx].status = celld_session::StepStatus::InProgress;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            // Simulate work: in a real agent, this would be the actual task execution
            workload.steps[next_idx].status = celld_session::StepStatus::Completed;
            workload.steps[next_idx].result = Some(result);
            workload.steps[next_idx].completed_at = Some(now);

            match client.put(&key, &serde_json::to_value(&workload).unwrap(), "workload") {
                Ok(meta) => json!({
                    "success": true,
                    "session": session,
                    "name": name,
                    "step_completed": workload.steps[next_idx].id,
                    "progress": workload.progress_summary(),
                    "complete": workload.is_complete(),
                    "meta": meta
                }),
                Err(e) => json!({
                    "success": false,
                    "error": e
                }),
            }
        }

        CellWorkloadCommand::Resume { session, name } => {
            let client = celld_session::SessionClient::new(&session);
            let key = format!("workload:{}", name);

            let value = match client.get(&key) {
                Ok(Some(v)) => v,
                Ok(None) => {
                    return json!({ "success": false, "error": "workload not found", "session": session, "name": name })
                }
                Err(e) => {
                    return json!({ "success": false, "error": e })
                }
            };

            let workload: celld_session::CheckpointedWorkload =
                match serde_json::from_value(value) {
                    Ok(w) => w,
                    Err(e) => {
                        return json!({ "success": false, "error": format!("invalid workload data: {}", e) })
                    }
                };

            if workload.is_complete() {
                return json!({
                    "success": true,
                    "session": session,
                    "name": name,
                    "complete": true,
                    "message": "workload already complete",
                    "steps": workload.steps
                });
            }

            // Resume: execute all remaining pending steps
            let mut current = workload;
            let mut completed_steps = Vec::new();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            while let Some(idx) = current.next_pending() {
                current.steps[idx].status = celld_session::StepStatus::Completed;
                current.steps[idx].result = Some("resumed".to_string());
                current.steps[idx].completed_at = Some(now);
                completed_steps.push(current.steps[idx].id.clone());

                // Checkpoint after each step (this is the key: state survives interruption)
                if let Err(e) = client.put(&key, &serde_json::to_value(&current).unwrap(), "workload") {
                    return json!({
                        "success": false,
                        "error": format!("checkpoint failed at {}: {}", current.steps[idx].id, e),
                        "completed_so_far": completed_steps
                    });
                }
            }

            json!({
                "success": true,
                "session": session,
                "name": name,
                "resumed": true,
                "steps_completed": completed_steps,
                "progress": current.progress_summary(),
                "complete": current.is_complete()
            })
        }
    }
}
