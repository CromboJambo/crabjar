/// Execute dinitctl command, routing through crabjar-dinit instance when configured.
pub fn execute_via_dinitctl(
    socket_path: &Option<String>,
    args: &[String],
    cwd: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let effective_socket = match socket_path {
        Some(s) => s.clone(),
        None => "/var/run/dinit.sock".to_string(),
    };

    // Build dinitctl args with crabjar-dinit socket
    let mut dinit_args: Vec<String> = vec!["-p".to_string(), effective_socket.clone()];
    dinit_args.extend(args.iter().cloned());
    let dinit_args: Vec<&str> = dinit_args.iter().map(|s| s.as_str()).collect();

    let output = std::process::Command::new("dinitctl")
        .args(&dinit_args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("dinitctl failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(stderr.trim().to_string().into());
    }

    // Parse exit code from output if present
    let exit_code = stdout
        .lines()
        .find_map(|line| line.trim().parse::<i32>().ok())
        .unwrap_or(0);

    Ok(exit_code)
}
