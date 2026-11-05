use crate::service::{ServiceConfig, ServiceManager, ServiceResult, ServiceStatus};
use std::fs;
use std::path::Path;

/// Systemd service manager implementation.
pub struct SystemdManager {
    user_mode: bool,
}

impl Default for SystemdManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemdManager {
    #[must_use]
    pub fn new() -> Self {
        // Detect if we're in user or system mode
        let user_mode = std::env::var("XDG_RUNTIME_DIR").is_ok();
        Self { user_mode }
    }

    fn systemctl_cmd(&self) -> String {
        if self.user_mode {
            "systemctl --user".to_string()
        } else {
            "systemctl".to_string()
        }
    }

    fn service_file_path(&self, name: &str) -> String {
        if self.user_mode {
            let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
                format!("{}/.config", std::env::var("HOME").unwrap_or_default())
            });
            format!("{xdg}/systemd/user/{name}.service")
        } else {
            format!("/etc/systemd/system/{name}.service")
        }
    }

    fn generate_systemd_unit(config: &ServiceConfig) -> String {
        let mut lines = Vec::new();
        lines.push("[Unit]".to_string());
        lines.push(format!("Description=Crabjar service: {}", config.name));
        lines.push("After=network.target".to_string());
        lines.push(String::new());

        lines.push("[Service]".to_string());
        lines.push("Type=simple".to_string());

        if let Some(dir) = &config.working_dir {
            lines.push(format!("WorkingDirectory={dir}"));
        }

        // Build command with args
        let mut cmd = config.command.clone();
        for arg in &config.args {
            cmd.push(' ');
            cmd.push_str(arg);
        }
        lines.push(format!("ExecStart={cmd}"));

        // Environment variables
        for (key, value) in &config.environment {
            lines.push(format!("Environment={key}={value}"));
        }

        // Restart policy
        match config.restart_policy {
            crate::service::RestartPolicy::Always => {
                lines.push("Restart=always".to_string());
            }
            crate::service::RestartPolicy::OnFailure => {
                lines.push("Restart=on-failure".to_string());
            }
            crate::service::RestartPolicy::Never => {
                lines.push("Restart=no".to_string());
            }
        }

        if let Some(log) = &config.log_file {
            lines.push(format!("StandardOutput=append:{log}"));
            lines.push(format!("StandardError=append:{log}"));
        }

        lines.push(String::new());
        lines.push("[Install]".to_string());
        lines.push("WantedBy=default.target".to_string());

        lines.join("\n") + "\n"
    }
}

impl ServiceManager for SystemdManager {
    fn start(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} start {name}"))
    }

    fn stop(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} stop {name}"))
    }

    fn restart(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} restart {name}"))
    }

    fn status(&self, name: &str) -> ServiceStatus {
        let cmd = self.systemctl_cmd();
        let result = crate::service::exec_command(&format!("{cmd} is-active {name}"));
        if result.success && result.output.trim() == "active" {
            ServiceStatus::Running
        } else if result.success && result.output.trim() == "inactive" {
            ServiceStatus::Stopped
        } else {
            ServiceStatus::Unknown
        }
    }

    fn enable(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} enable {name}"))
    }

    fn disable(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} disable {name}"))
    }

    fn list(&self) -> Vec<String> {
        let cmd = self.systemctl_cmd();
        let result =
            crate::service::exec_command(&format!("{cmd} list-units --type=service --no-pager"));
        if !result.success {
            return Vec::new();
        }

        result
            .output
            .lines()
            .filter(|line| line.contains(".service"))
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn deploy(&self, config: &ServiceConfig) -> ServiceResult {
        let path = self.service_file_path(&config.name);
        let content = Self::generate_systemd_unit(config);

        // Ensure directory exists
        if let Some(parent) = Path::new(&path).parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return ServiceResult::err(format!("Failed to create directory: {e}"), -1);
        }

        // Write service file
        if let Err(e) = fs::write(&path, content) {
            return ServiceResult::err(format!("Failed to write service file: {e}"), -1);
        }

        // Reload systemd daemon
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} daemon-reload"))
    }

    fn reload(&self, name: &str) -> ServiceResult {
        let cmd = self.systemctl_cmd();
        crate::service::exec_command(&format!("{cmd} reload {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let mgr = SystemdManager::new();
        // Should not panic
        let _ = mgr.systemctl_cmd();
    }

    #[test]
    fn test_generate_unit_basic() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_args(vec!["arg1".to_string(), "arg2".to_string()]);

        let unit = SystemdManager::generate_systemd_unit(&config);
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("ExecStart=/usr/bin/test arg1 arg2"));
    }

    #[test]
    fn test_generate_unit_restart_always() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_restart_policy(crate::service::RestartPolicy::Always);

        let unit = SystemdManager::generate_systemd_unit(&config);
        assert!(unit.contains("Restart=always"));
    }
}
