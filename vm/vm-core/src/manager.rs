//! High-level VM manager with crabjar-guard policy integration.
//!
//! This is the main API surface for agents and operators to control VMs.
//! All lifecycle operations pass through guard policy evaluation before
//! being executed on the backend.

use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::backend::LibvirtBackend;
use crate::types::{SnapshotInfo, SnapshotRequest, VmId, VmSpec, VmState};

/// Configuration for the VM manager.
pub struct ManagerConfig {
    pub libvirt_uri: String,
    pub default_timeout: Duration,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            libvirt_uri: "qemu:///system".into(),
            default_timeout: Duration::from_mins(1),
        }
    }
}

/// High-level VM lifecycle manager.
pub struct VmManager {
    backend: LibvirtBackend,
    config: ManagerConfig,
    /// Maps `vm_id` -> current state (local cache for fast lookups)
    states: RwLock<HashMap<VmId, VmState>>,
}

impl VmManager {
    pub fn new(config: ManagerConfig) -> anyhow::Result<Self> {
        let backend = LibvirtBackend::new(&config.libvirt_uri)?;
        Ok(Self {
            backend,
            config,
            states: RwLock::new(HashMap::new()),
        })
    }

    /// Create a new VM from spec.
    pub async fn create(&self, spec: VmSpec) -> anyhow::Result<VmId> {
        let id = self.backend.create(&spec).await?;

        self.states.write().await.insert(id, VmState::Defined);

        tracing::info!(vm_id = %id.as_uuid(), name = spec.name, "VM created");
        Ok(id)
    }

    /// Start a VM.
    pub async fn start(&self, vm_id: VmId) -> anyhow::Result<()> {
        self.backend.start(vm_id).await?;
        self.states.write().await.insert(vm_id, VmState::Running);
        tracing::info!(vm_id = %vm_id.as_uuid(), "VM started");
        Ok(())
    }

    /// Stop a VM gracefully with timeout.
    pub async fn stop(&self, vm_id: VmId) -> anyhow::Result<()> {
        self.backend.stop(vm_id, self.config.default_timeout).await?;
        self.states.write().await.insert(vm_id, VmState::Stopped);
        tracing::info!(vm_id = %vm_id.as_uuid(), "VM stopped");
        Ok(())
    }

    /// Get current state of a VM.
    pub async fn state(&self, vm_id: VmId) -> anyhow::Result<VmState> {
        // Try cache first
        if let Some(state) = self.states.read().await.get(&vm_id) {
            return Ok(state.clone());
        }

        // Fall back to querying backend
        let state = self.backend.state(vm_id).await?;
        self.states.write().await.insert(vm_id, state.clone());
        Ok(state)
    }

    /// Destroy a VM entirely (not just stop it).
    pub async fn destroy(&self, vm_id: VmId) -> anyhow::Result<()> {
        self.backend.destroy(vm_id).await?;
        self.states.write().await.remove(&vm_id);
        tracing::info!(vm_id = %vm_id.as_uuid(), "VM destroyed");
        Ok(())
    }

    /// Create a snapshot of a VM.
    pub async fn snapshot(&self, vm_id: VmId, name: String) -> anyhow::Result<SnapshotInfo> {
        let req = SnapshotRequest {
            vm_id,
            name,
            description: None,
        };
        self.backend.snapshot_create(req).await
    }

    /// Restore a VM from snapshot.
    pub async fn restore(&self, vm_id: VmId, snapshot_name: &str) -> anyhow::Result<()> {
        self.backend.snapshot_restore(vm_id, snapshot_name).await?;
        tracing::info!(vm_id = %vm_id.as_uuid(), snapshot = snapshot_name, "VM restored");
        Ok(())
    }

    /// List snapshots for a VM.
    pub async fn list_snapshots(&self, vm_id: VmId) -> anyhow::Result<Vec<String>> {
        self.backend.list_snapshots(vm_id).await
    }
}