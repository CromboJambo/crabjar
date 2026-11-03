use crate::service::{ServiceConfig, ServiceManager, ServiceResult, ServiceStatus};
use std::fs;
use std::path::Path;

/// Runit service manager implementation.
pub struct RunitManager {
    sv_dir: String,
}

impl Default for RunitManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RunitManager {
    #[must_use]
    pub fn new() -> Self {
        // Detect runit service directory
        if Path::new("/etc/sv").exists() {
            Self {
                sv_dir: "/etc/sv".to_string(),
            }
        } else {
            Self {
                sv_dir: "/var/service".to_string(),
            }
        }
    }

    fn generate_runit_script(config: &ServiceConfig) -> String {
        let mut lines = Vec::new();
        lines.push("#!/bin/sh".to_string());

        if let Some(dir) = &config.working_dir {
            lines.push(format!("cd {dir}"));
        }

        // Build command with args
        let mut cmd = config.command.clone();
        for arg in &config.args {
            cmd.push(' ');
            cmd.push_str(arg);
        }
        lines.push(format!("exec {cmd}"));

        lines.join("\n") + "\n"
    }

    fn generate_run_file_path(&self, name: &str) -> String {
        format!("{}/{}/run", self.sv_dir, name)
    }
}

impl ServiceManager for RunitManager {
    fn start(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("sv start {name}"))
    }

    fn stop(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("sv stop {name}"))
    }

    fn restart(&self, name: &str) -> ServiceResult {
        crate::service::exec_command(&format!("sv restart {name}"))
    }

    fn status(&self, name: &str) -> ServiceStatus {
        let result = crate::service::exec_command(&format!("sv status {name}"));
        if result.success && result.output.contains("running") {
            ServiceStatus::Running
        } else if result.success && result.output.contains("stopped") {
            ServiceStatus::Stopped
        } else {
            ServiceStatus::Unknown
        }
    }

    fn enable(&self, name: &str) -> ServiceResult {
        // In runit, services are enabled by default if they exist in the sv dir
        self.start(name)
    }

    fn disable(&self, name: &str) -> ServiceResult {
        // Disable by renaming the service directory (runit convention)
        crate::service::exec_command(&format!(
            "mv {} {}",
            format!("{}/{}", self.sv_dir, name),
            format!("{}/{}.disabled", self.sv_dir, name)
        ))
    }

    fn list(&self) -> Vec<String> {
        let result = crate::service::exec_command("sv list");
        if !result.success {
            return Vec::new();
        }

        result
            .output
            .lines()
            .filter(|line| line.contains("running") || line.contains("stopped"))
            .map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    parts[1].trim_end_matches('/').to_string()
                } else {
                    String::new()
                }
            })
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn deploy(&self, config: &ServiceConfig) -> ServiceResult {
        let service_dir = format!("{}/{}", self.sv_dir, config.name);

        // Create service directory
        if let Err(e) = fs::create_dir_all(&service_dir) {
            return ServiceResult::err(
                format!("Failed to create service directory: {e}"),
                -1,
            );
        }

        // Write run script
        let run_path = self.generate_run_file_path(&config.name);
        let run_script = Self::generate_runit_script(config);

        if let Err(e) = fs::write(&run_path, run_script) {
            return ServiceResult::err(
                format!("Failed to write run script: {e}"),
                -1,
            );
        }

        // Make run script executable
        crate::service::exec_command(&format!("chmod +x {run_path}"))
    }

    fn reload(&self, name: &str) -> ServiceResult {
        self.restart(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let mgr = RunitManager::new();
        assert!(!mgr.sv_dir.is_empty());
    }

    #[test]
    fn test_generate_run_script() {
        let config = ServiceConfig::new("test-service", "/usr/bin/test")
            .with_args(vec!["arg1".to_string(), "arg2".to_string()]);

        let script = RunitManager::generate_runit_script(&config);
        assert!(script.contains("#!/bin/sh"));
        assert!(script.contains("exec /usr/bin/test arg1 arg2"));
    }
}