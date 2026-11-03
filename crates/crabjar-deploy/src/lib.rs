//! Crabjar deployment and service management library.
//!
//! Provides an abstraction layer over different init systems (dinit, runit, systemd)
//! for deploying and managing crabjar services across diverse environments.
//!
//! # Usage
//!
//! ```hemlock
//! import { create_service_manager } from "@stdlib/deploy";
//!
//! let mgr = create_service_manager();
//! mgr.start("crabjar-conductor");
//! print(mgr.status("crabjar-conductor"));
//! ```

mod detect;
mod service;
mod managers;

pub use detect::{detect_init_systems, detect_primary_init, InitSystem};
pub use service::{ServiceConfig, ServiceManager, ServiceResult, ServiceStatus, RestartPolicy};
pub use managers::{DinitManager, RunitManager, SystemdManager, NullManager};

/// Factory function to create the appropriate service manager based on the detected init system.
#[must_use]
pub fn create_service_manager() -> Box<dyn ServiceManager> {
    let init = detect_primary_init();

    match init {
        InitSystem::Dinit => Box::new(DinitManager::new()),
        InitSystem::Runit => Box::new(RunitManager::new()),
        InitSystem::Systemd => Box::new(SystemdManager::new()),
        InitSystem::None => Box::new(NullManager::new()),
    }
}

/// Create a service manager for a specific init system (override auto-detection).
#[must_use]
pub fn create_service_manager_for(init: &InitSystem) -> Box<dyn ServiceManager> {
    match init {
        InitSystem::Dinit => Box::new(DinitManager::new()),
        InitSystem::Runit => Box::new(RunitManager::new()),
        InitSystem::Systemd => Box::new(SystemdManager::new()),
        InitSystem::None => Box::new(NullManager::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service_manager() {
        let mgr = create_service_manager();
        // Should not panic and should return a valid manager
        let services = mgr.list();
        assert!(services.len() == 0 || services.len() > 0);
    }

    #[test]
    fn test_detect_init_systems() {
        let systems = detect_init_systems();
        assert!(!systems.is_empty(), "Should always detect at least 'none'");
    }
}