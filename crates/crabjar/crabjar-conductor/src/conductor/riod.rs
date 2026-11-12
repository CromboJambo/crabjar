//! Resource-aware scheduling via riod.
//!
//! Queries riod's availability endpoint before task assignment to prevent
//! OOM kills and GPU starvation. Fails open (allows assignment) if riod is
//! unreachable — resource gating is an optimization, not a hard dependency.

use std::path::PathBuf;

use riod_client::RiodClient;
use tracing::{debug, warn};

use crate::task_spec::TaskResources;

/// Resource checker that validates task requirements against live availability.
pub struct RiodResourceChecker {
    client: Option<RiodClient>,
    enabled: bool,
}

impl Default for RiodResourceChecker {
    fn default() -> Self {
        Self {
            client: None,
            enabled: true,
        }
    }
}

impl RiodResourceChecker {
    /// Create a new checker pointing at the given riod socket.
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        let path = socket_path.into();
        if !path.exists() {
            debug!(
                "riod socket {} does not exist — resource checks disabled",
                path.display()
            );
            return Self::default();
        }

        match RiodClient::connect(&path) {
            Ok(client) => {
                debug!("Connected to riod at {}", path.display());
                Self {
                    client: Some(client),
                    enabled: true,
                }
            }
            Err(e) => {
                warn!(
                    "Failed to connect to riod at {}: {} — resource checks disabled",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Create a checker from an existing client (for testing).
    #[must_use]
    pub fn with_client(client: RiodClient) -> Self {
        Self {
            client: Some(client),
            enabled: true,
        }
    }

    /// Check if riod is available and the given resources can be satisfied.
    ///
    /// Returns `true` if:
    /// - riod is unreachable (fail-open)
    /// - riod reports sufficient RAM, CPU headroom, and GPU availability
    ///
    /// Returns `false` only when riod is reachable AND explicitly says resources are insufficient.
    pub fn can_satisfy(&self, resources: &TaskResources) -> bool {
        if !self.enabled {
            return true; // Fail open
        }

        let Some(client) = &self.client else {
            return true; // No client, fail open
        };

        match client.available_resources() {
            Ok(avail) => evaluate(&avail, resources),
            Err(e) => {
                debug!(
                    "riod availability check failed: {} — allowing assignment",
                    e
                );
                true // Fail open on transient errors
            }
        }
    }

    /// Query current resource availability (for monitoring/debugging).
    #[must_use]
    pub fn query_availability(&self) -> Option<riod_client::Availability> {
        self.client.as_ref()?.available_resources().ok()
    }
}

/// Evaluate resource requirements against live availability.
fn evaluate(avail: &riod_client::Availability, resources: &TaskResources) -> bool {
    // RAM check: task needs memory_mb, riod reports available in MB (u64)
    if u64::from(resources.memory_mb) > avail.available_ram_mb {
        debug!(
            "riod: insufficient RAM — need {}MB, available {}MB",
            resources.memory_mb, avail.available_ram_mb
        );
        return false;
    }

    // GPU check: task needs a GPU, riod reports available GPU indices
    if resources.gpu && avail.available_gpus.is_empty() {
        debug!("riod: no GPUs available — task requires GPU");
        return false;
    }

    // CPU check is soft: we don't have live CPU load from riod's availability
    // endpoint, so we skip it. The cgroup enforcement will handle actual limits.

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fails_open_when_disabled() {
        let checker = RiodResourceChecker::default();
        let resources = TaskResources {
            cpu: 8,
            memory_mb: 65536,
            gpu: true,
            network: true,
        };
        // No client, no riod — should allow everything
        assert!(checker.can_satisfy(&resources));
    }

    #[test]
    fn insufficient_ram_blocks() {
        let avail = riod_client::Availability {
            available_gpus: vec![0],
            total_gpus: 1,
            allocated_gpus: 0,
            total_ram_mb: 16384,
            allocated_ram_mb: 12288,
            available_ram_mb: 4096,
            active_allocations: 1,
        };

        // Task needs 8GB but only 4GB available
        let resources = TaskResources {
            cpu: 2,
            memory_mb: 8192,
            gpu: false,
            network: true,
        };
        assert!(!evaluate(&avail, &resources));

        // Task needs 2GB and 4GB available — OK
        let resources_ok = TaskResources {
            cpu: 2,
            memory_mb: 2048,
            gpu: false,
            network: true,
        };
        assert!(evaluate(&avail, &resources_ok));
    }

    #[test]
    fn no_gpu_blocks_gpu_task() {
        let avail = riod_client::Availability {
            available_gpus: vec![],
            total_gpus: 1,
            allocated_gpus: 1,
            total_ram_mb: 16384,
            allocated_ram_mb: 0,
            available_ram_mb: 16384,
            active_allocations: 1,
        };

        let gpu_task = TaskResources {
            cpu: 2,
            memory_mb: 4096,
            gpu: true,
            network: true,
        };
        assert!(!evaluate(&avail, &gpu_task));

        let cpu_task = TaskResources {
            cpu: 2,
            memory_mb: 4096,
            gpu: false,
            network: true,
        };
        assert!(evaluate(&avail, &cpu_task));
    }
}
