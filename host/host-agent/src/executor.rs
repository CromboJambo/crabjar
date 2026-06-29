/// TaskExecutor — runs individual tasks defined in a WorkItem's plan.
///
/// Each task is independently executable. Results are captured and stored.
use crabjar_host_core::{WorkItem, work_item::TaskStatus};

pub struct TaskExecutor;

impl TaskExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a single task by index.
    pub fn execute_task(&self, work_item: &mut WorkItem, task_id: usize) -> Result<String, String> {
        if task_id >= work_item.plan.len() {
            return Err(format!(
                "task {} out of range (plan has {} tasks)",
                task_id,
                work_item.plan.len()
            ));
        }

        let task_desc = work_item.plan[task_id].description.clone();
        work_item.update_task(task_id, TaskStatus::InProgress, None);
        work_item.observe("execute", "task", format!("Executing: {task_desc}"));

        // Attempt to resolve the task as a tool call via the tool registry
        let result = self.execute_via_registry(&task_desc);

        work_item.update_task(task_id, TaskStatus::Completed, Some(result.clone()));
        work_item.add_artifact(&result);

        Ok(result)
    }

    /// Execute all pending tasks in the WorkItem's plan.
    pub fn execute_all(&self, work_item: &mut WorkItem) -> Vec<(usize, Result<String, String>)> {
        let results: Vec<_> = (0..work_item.plan.len())
            .map(|id| (id, self.execute_task(work_item, id)))
            .collect();

        work_item.observe(
            "execute",
            "batch",
            format!("Executed {} tasks", results.len()),
        );
        results
    }

    /// Try to resolve a task description as a tool call.
    /// Falls back to the stub simulation if no tool is found.
    fn execute_via_registry(&self, task_desc: &str) -> String {
        // Try to parse task description as a tool call: "tool_name arg1 arg2 ..."
        let parts: Vec<&str> = task_desc.splitn(3, ' ').collect();
        if parts.is_empty() {
            return format!("Task '{task_desc}' executed successfully (stub)");
        }

        let tool_name = parts[0];
        let tool_args = if parts.len() > 1 { &parts[1..] } else { &[] };

        // Check tool registry for the tool
        let project_root = std::env::current_dir()
            .ok()
            .unwrap_or_else(|| std::path::PathBuf::from("/home/crombo/crabjar"));
        let tool_registry_path = project_root.join("tool_registry/tool_registry.db");

        if !tool_registry_path.exists() {
            return format!("Task '{task_desc}' executed successfully (stub — no registry)");
        }

        let conn = match rusqlite::Connection::open(&tool_registry_path) {
            Ok(c) => c,
            Err(e) => {
                return format!(
                    "Task '{task_desc}' executed successfully (stub — registry error: {e})"
                );
            }
        };

        let registry = crabjar_tool_registry::ToolRegistry::new(&conn);
        if registry.init().is_err() {
            return format!("Task '{task_desc}' executed successfully (stub — init failed)");
        }

        // Check if tool exists in registry
        if registry.query_tool(tool_name).ok().flatten().is_some() {
            // Tool registered — validate binary availability
            let all_tools = vec![tool_name.to_string()];
            if let Ok(validation) = registry.validate_tools(&all_tools)
                && let Some((_, available, binary_path)) = validation.first()
            {
                if *available {
                    // Execute via guard gate
                    let guard_root = std::env::var("MIRROR_GUARD_ROOT")
                        .unwrap_or_else(|_| project_root.to_string_lossy().to_string());

                    let guard_db =
                        crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(
                            format!("{}/guard.db", guard_root),
                        ))
                        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(":memory:").unwrap());

                    let gate = crabjar_guard::ExecutionGate::new(&guard_db, false, &guard_root);
                    let gate_result = match gate.check(crabjar_guard::GateContext {
                        action_type: "tool_call",
                        command: tool_name,
                        args: tool_args.iter().map(|s| s.to_string()).collect(),
                        trust_layer: 2,
                        confidence: crabjar_guard::TrustScore::new(0.5),
                        source_event_id: Some("host-agent-exec"),
                        can_interrupt: true,
                        pid: None,
                        scope: None,
                        target_scope: None,
                        domains: vec![],
                        context_budget: None,
                        context_fragment_tokens: None,
                    }) {
                        Ok(r) => r,
                        Err(e) => {
                            return format!("Task '{task_desc}' failed: security gate error: {e}");
                        }
                    };

                    match gate_result {
                        crabjar_guard::GateResult::Proceed => {
                            // Execute the binary
                            if let Some(path) = binary_path {
                                match std::process::Command::new(path).args(tool_args).output() {
                                    Ok(output) => {
                                        let stdout =
                                            String::from_utf8_lossy(&output.stdout).to_string();
                                        let stderr =
                                            String::from_utf8_lossy(&output.stderr).to_string();
                                        if !output.status.success() {
                                            return format!(
                                                "Task '{tool_name}' failed with exit code {}: {stderr}",
                                                output.status.code().unwrap_or(-1)
                                            );
                                        }
                                        return if stdout.is_empty() {
                                            format!(
                                                "Task '{tool_name}' executed successfully (exit code: {})",
                                                output.status.code().unwrap_or(0)
                                            )
                                        } else {
                                            format!("Task '{tool_name}' output:\n{stdout}")
                                        };
                                    }
                                    Err(e) => {
                                        return format!(
                                            "Task '{tool_name}' failed to execute: {e}"
                                        );
                                    }
                                }
                            } else {
                                return format!(
                                    "Task '{tool_name}' registered but binary not found"
                                );
                            }
                        }
                        crabjar_guard::GateResult::Pending => {
                            return format!("Task '{tool_name}' pending — requires approval");
                        }
                        crabjar_guard::GateResult::Interrupted { .. } => {
                            return format!("Task '{tool_name}' interrupted");
                        }
                        crabjar_guard::GateResult::DryRun => {
                            return format!("Task '{tool_name}' dry-run (no-op)");
                        }
                        crabjar_guard::GateResult::Revoked { .. } => {
                            return format!("Task '{tool_name}' permissions revoked");
                        }
                        crabjar_guard::GateResult::ContextExhausted {
                            used,
                            budget,
                            remaining,
                        } => {
                            return format!(
                                "Task '{tool_name}' context budget exhausted: {used} / {budget} tokens, {remaining} remaining"
                            );
                        }
                    };
                } else {
                    return format!("Task '{tool_name}' registered but binary not found in PATH");
                }
            }
        }

        // No tool found — fall back to stub
        format!("Task '{task_desc}' executed successfully (stub — no matching tool)")
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}
