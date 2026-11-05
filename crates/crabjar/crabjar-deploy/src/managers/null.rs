use crate::service::{ServiceConfig, ServiceManager, ServiceResult, ServiceStatus};

/// Null service manager for environments without an init system (e.g., containers).
/// Services are managed by writing PID files and tracking them in memory.
pub struct NullManager {
    pid_dir: String,
}

impl Default for NullManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NullManager {
    #[must_use]
    pub fn new() -> Self {
        let pid_dir = std::env::var("PID_DIR").unwrap_or_else(|_| "/run".to_string());
        Self { pid_dir }
    }

    fn pid_file_path(&self, name: &str) -> String {
        format!("{}/{}.pid", self.pid_dir, name)
    }
}

impl ServiceManager for NullManager {
    fn start(&self, name: &str) -> ServiceResult {
        // Check if already running by reading PID file
        let pid_path = self.pid_file_path(name);
        if std::fs::exists(&pid_path).unwrap_or(false) {
            return ServiceResult::ok(format!("Service {name} already running"));
        }

        // For null manager, we just record that the service should be started
        // The actual process management would be done by the caller
        ServiceResult::ok(format!("Service {name} marked for start"))
    }

    fn stop(&self, name: &str) -> ServiceResult {
        let pid_path = self.pid_file_path(name);
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            let pid: i32 = pid_str.trim().parse().unwrap_or(-1);
            if pid > 0 {
                return crate::service::exec_command(&format!("kill {pid}"));
            }
        }

        // Remove PID file
        if std::fs::exists(&pid_path).unwrap_or(false) {
            let _ = std::fs::remove_file(&pid_path);
        }

        ServiceResult::ok(format!("Service {name} stopped"))
    }

    fn restart(&self, name: &str) -> ServiceResult {
        let stop_result = self.stop(name);
        if !stop_result.success {
            return stop_result;
        }
        self.start(name)
    }

    fn status(&self, name: &str) -> ServiceStatus {
        let pid_path = self.pid_file_path(name);
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            let pid: i32 = pid_str.trim().parse().unwrap_or(-1);
            if pid > 0 {
                // Check if process is still running
                let result = crate::service::exec_command(&format!("kill -0 {pid}"));
                if result.success {
                    return ServiceStatus::Running;
                }
            }
        }
        ServiceStatus::Stopped
    }

    fn enable(&self, _name: &str) -> ServiceResult {
        // No persistent enable in null manager
        ServiceResult::ok("Enabled (no-op in null manager)".to_string())
    }

    fn disable(&self, _name: &str) -> ServiceResult {
        // No persistent disable in null manager
        ServiceResult::ok("Disabled (no-op in null manager)".to_string())
    }

    fn list(&self) -> Vec<String> {
        let mut services = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.pid_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".pid") {
                    services.push(name.trim_end_matches(".pid").to_string());
                }
            }
        }
        services
    }

    fn deploy(&self, config: &ServiceConfig) -> ServiceResult {
        // In null manager, deployment just means recording the service config
        // The actual process management is done by the caller
        let pid_path = self.pid_file_path(&config.name);

        // Write a marker file indicating the service should exist
        if let Err(e) = std::fs::write(
            &pid_path,
            format!("service: {}\ncommand: {}", config.name, config.command),
        ) {
            return ServiceResult::err(format!("Failed to write PID file: {e}"), -1);
        }

        ServiceResult::ok(format!("Service {} deployed (null manager)", config.name))
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
        let mgr = NullManager::new();
        assert!(!mgr.pid_dir.is_empty());
    }
}
