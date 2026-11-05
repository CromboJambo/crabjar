//! Plugin context: environment available to plugins during lifecycle.
//!
//! Pending integration — not yet wired into the plugin lifecycle flow.

use crabjar_guard::{ExecutionGate, GuardDb};
use std::path::PathBuf;

/// Context available to plugins during their lifecycle.
#[expect(dead_code)]
pub struct PluginContext {
    pub cwd: PathBuf,
    pub guard_db: GuardDb,
    /// Optional telemetry handle for recording plugin invocations.
    #[allow(dead_code)]
    pub telemetry_handle: Option<String>, // Simplified; would be TelemetryHandle in full impl
}

#[expect(dead_code)]
impl PluginContext {
    pub fn new(cwd: PathBuf, guard_db: GuardDb) -> Self {
        Self {
            cwd,
            guard_db,
            telemetry_handle: None,
        }
    }

    /// Create a gate for tool authorization within this context.
    pub fn create_gate(&self) -> ExecutionGate<'_> {
        ExecutionGate::new(
            &self.guard_db,
            false,
            self.cwd.to_string_lossy().to_string(),
        )
    }
}
