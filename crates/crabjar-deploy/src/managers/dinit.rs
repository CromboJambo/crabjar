use crate::service::{ServiceConfig, ServiceManager, ServiceResult, ServiceStatus};
use std::fs;
use std::path::Path;

/// Dinit service manager implementation.
pub struct DinitManager {
    config_dir: String,
}

impl Default for DinitManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DinitManager {
    #[must_use]
    pub fn new() -> Self {
        // Detect dinit config directory
        if Path::new("/etc/dinit.d").exists() {
            Self {
                config_dir: "/etc/dinit.d".to_string(),
            }
        } else {
            Self {
                config_dir: "/var/dinit.d".to_string(),
            }
        }
    }

    fn service_file_path(&self, name: &str) -> String {
        format!("{}/{}", self.config_dir, name)
    }

    fn generate_dinit_config(config: &ServiceConfig) -> String {
        let mut lines = Vec::new();
        lines.push("type     = process".to_string());

        // Build command with args
        let mut cmd = config.command.clone();
        for arg in &config.args {
            cmd.push(' ');
            cmd.push_str(arg);
        }
        lines.push(format!("command  = {cmd}"));

        if let Some(dir) = &config.working_dir {
            lines.push(format!("directory= {dir}"));
        }

        // Environment variables
        for (key, value) in &config.environment {
            lines.push(format!("env      = {key}={value}"));
        }

        if let Some(log) = &config.log_file {
            lines.push(format!("logfile  = {log}"));
        }

        // Restart policy
        match config.restart_policy {
            crate::service::RestartPolicy::Always => {
                lines.push("respawn  = true".to_string());
            }
            crate::service::RestartPolicy::OnFailure => {
                lines.push("respawn  = on-failure".to_string());
            }
            crate::service::RestartPolicy::Never => {
                // No respawn line
            }
        }

        lines.join("\n") + "\n"
    }
}

impl ServiceManager for DinitManager {
    fn start(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("dinitctl start {name}"))
    }

    fn stop(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("dinitctl stop {name}"))
    }

    fn restart(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("dinitctl restart {name}"))
    }

    fn status(&self, name: &str) -> ServiceStatus {
        let result = crate::service::exec_command(&format!("dinitctl status {name}"));
        if result.success && result.output.contains("running") {
            ServiceStatus::Running
        } else if result.success && result.output.contains("stopped") {
            ServiceStatus::Stopped
        } else {
            ServiceStatus::Unknown
        }
    }

    fn enable(&self, name: &str) -> ServiceResult {
        // In dinit, services are enabled by default if they exist in the config dir
        self.start(name)
    }

    fn disable(&self, name: &str) -> ServiceResult {
        self.stop(name)
    }

    fn list(&self) -> Vec<String> {
        let result = crate::service::exec_command("dinitctl list");
        if !result.success {
            return Vec::new();
        }

        result
            .output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }

    fn deploy(&self, config: &ServiceConfig) -> ServiceResult {
        let path = self.service_file_path(&config.name);
        let content = Self::generate_dinit_config(config);

        // Write service file
        match fs::write(&path, content) {
            Ok(()) => {
                // Reload dinit to pick up the new service
                crate::service::exec_command(&format!("dinitctl reload {}", config.name))
            }
            Err(e) => ServiceResult::err(format!("Failed to write service file: {e}"), -1),
        }
    }

    fn reload(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("dinitctl reload {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let mgr = DinitManager::new();
        // Should not panic
        assert!(!mgr.config_dir.is_empty());
    }

    #[test]
    fn test_generate_config_basic() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_args(vec!["arg1".to_string(), "arg2".to_string()]);

        let generated = DinitManager::generate_dinit_config(&config);
        assert!(generated.contains("command  = /usr/bin/test arg1 arg2"));
        assert!(generated.contains("type     = process"));
    }

    #[test]
    fn test_generate_config_with_log() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_log_file("/var/log/test.log");

        let generated = DinitManager::generate_dinit_config(&config);
        assert!(generated.contains("logfile  = /var/log/test.log"));
    }

    #[test]
    fn test_generate_config_restart_always() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_restart_policy(crate::service::RestartPolicy::Always);

        let generated = DinitManager::generate_dinit_config(&config);
        assert!(generated.contains("respawn  = true"));
    }
}