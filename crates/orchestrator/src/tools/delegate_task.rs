use crabjar_delegation::{DelegationAction, DelegationInput};

pub async fn handle(args: &[String], _ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: delegate_task requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: DelegationInput = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing delegate_task arguments: {}", e))?;

    // Create dispatcher with default config
    let dispatcher = crabjar_delegation::Dispatcher::new(None);

    match input.action {
        DelegationAction::Spawn => {
            if input.tasks.is_empty() {
                return Err("Error: delegate_task spawn requires at least one task".to_string());
            }

            let mut results = Vec::new();
            for task in &input.tasks {
                match dispatcher.execute_task(task.goal.clone(), task.context.clone()).await {
                    Ok(task_id) => {
                        results.push(serde_json::json!({
                            "status": "spawned",
                            "task_id": task_id,
                            "goal": task.goal
                        }));
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "status": "error",
                            "goal": task.goal,
                            "error": e.to_string()
                        }));
                    }
                }
            }

            serde_json::to_string_pretty(&results)
                .map_err(|e| format!("Error serializing spawn results: {}", e))
        }
        DelegationAction::List => match dispatcher.list_tasks().await {
            Ok(tasks) => {
                let results: Vec<serde_json::Value> = tasks
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "task_id": t.task_id,
                            "status": format!("{:?}", t.status),
                            "goal": &t.goal[..t.goal.len().min(100)],
                            "created_at": t.created_at.to_rfc3339()
                        })
                    })
                    .collect();

                serde_json::to_string_pretty(&results)
                    .map_err(|e| format!("Error serializing task list: {}", e))
            }
            Err(e) => Err(format!("Error listing tasks: {}", e)),
        },
        DelegationAction::Steer => {
            if input.subagent_id.is_none() || input.message.is_none() {
                return Err("Error: delegate_task steer requires subagent_id and message".to_string());
            }

            let task_id = input.subagent_id.unwrap();
            let message = input.message.unwrap();

            match dispatcher.steer_task(&task_id, &message).await {
                Ok(()) => Ok(format!("Steered task {}", task_id)),
                Err(e) => Err(format!("Error steering task: {}", e)),
            }
        }
        DelegationAction::Stop => {
            if input.subagent_id.is_none() {
                return Err("Error: delegate_task stop requires subagent_id".to_string());
            }

            let task_id = input.subagent_id.unwrap();

            match dispatcher.stop_task(&task_id).await {
                Ok(()) => Ok(format!("Stopped task {}", task_id)),
                Err(e) => Err(format!("Error stopping task: {}", e)),
            }
        }
    }
}
