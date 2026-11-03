use serde_json::json;

/// Execute command in isolated rootless Podman container.
pub fn execute_in_podman_container(
    command: &str,
    args: &[String],
    cwd: &str,
) -> Option<serde_json::Value> {
    let podman = match crabjar_podman::PodmanExecutor::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Warning: Podman not available: {}", e);
            return None;
        }
    };

    // Build sandbox config for isolated execution
    let agent_name = format!(
        "agent-{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    );
    let mut sandbox = crabjar_podman::SandboxConfig::new(&agent_name, vec![command.to_string()]);
    sandbox.command.extend(args.iter().cloned());
    sandbox.working_dir = cwd.to_string();

    // Create container
    match podman.create_container(&sandbox) {
        Ok(container_id) => {
            eprintln!("Created isolated Podman container: {}", container_id);

            // Start and wait for completion
            let result = podman.start_container(&container_id);

            // Clean up: stop and remove container
            if let Err(e) = podman.stop_container(&container_id) {
                eprintln!("Failed to stop container: {}", e);
            }
            if let Err(e) = podman.remove_container(&container_id) {
                eprintln!("Failed to remove container: {}", e);
            }

            match result {
                Ok(_) => Some(json!({
                    "container_id": container_id,
                    "status": "completed",
                    "exit_code": 0,
                })),
                Err(e) => Some(json!({
                    "container_id": container_id,
                    "status": "failed",
                    "error": e.to_string(),
                })),
            }
        }
        Err(e) => {
            eprintln!("Failed to create Podman container: {}", e);
            Some(json!({
                "container_id": null,
                "status": "creation_failed",
                "error": e.to_string(),
            }))
        }
    }
}
