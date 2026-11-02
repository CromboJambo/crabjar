use std::collections::HashMap;
use std::process::Command;

/// Service status states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Restarting,
    Unknown,
}

/// Result of a service operation.
#[derive(Debug)]
pub struct ServiceResult {
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
}

impl ServiceResult {
    pub fn ok(output: String) -> Self {
        Self {
            success: true,
            output,
            exit_code: None,
        }
    }

    pub fn err(output: String, exit_code: i32) -> Self {
        Self {
            success: false,
            output,
            exit_code: Some(exit_code),
        }
    }
}

/// Service configuration for deployment.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub environment: HashMap<String, String>,
    pub restart_policy: RestartPolicy,
    pub log_file: Option<String>,
}

/// Restart policy for services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

impl ServiceConfig {
    pub fn new(name: &str, command: &str) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            args: Vec::new(),
            working_dir: None,
            environment: HashMap::new(),
            restart_policy: RestartPolicy::OnFailure,
            log_file: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = Some(dir.to_string());
        self
    }

    pub fn with_log_file(mut self, path: &str) -> Self {
        self.log_file = Some(path.to_string());
        self
    }

    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }
}

/// Abstract service manager interface.
pub trait ServiceManager {
    /// Start a service by name.
    fn start(&self, name: &str) -> ServiceResult;

    /// Stop a service by name.
    fn stop(&self, name: &str) -> ServiceResult;

    /// Restart a service by name.
    fn restart(&self, name: &str) -> ServiceResult;

    /// Get the status of a service.
    fn status(&self, name: &str) -> ServiceStatus;

    /// Enable a service to start on boot.
    fn enable(&self, name: &str) -> ServiceResult;

    /// Disable a service from starting on boot.
    fn disable(&self, name: &str) -> ServiceResult;

    /// List all managed services.
    fn list(&self) -> Vec<String>;

    /// Deploy a service configuration (create/update service definition).
    fn deploy(&self, config: &ServiceConfig) -> ServiceResult;

    /// Reload service definitions from disk.
    fn reload(&self, name: &str) -> ServiceResult;
}

/// Execute a shell command and return the result.
pub fn exec_command(cmd: &str) -> ServiceResult {
    match Command::new("sh").args(["-c", cmd]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}{}", stdout, stderr);

            if output.status.success() {
                ServiceResult::ok(combined)
            } else {
                ServiceResult::err(
                    combined,
                    output.status.code().unwrap_or(-1),
                )
            }
        }
        Err(e) => ServiceResult::err(format!("Failed to execute: {}", e), -1),
    }
}