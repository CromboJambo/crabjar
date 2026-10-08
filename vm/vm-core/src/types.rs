//! Domain types for VM lifecycle management.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a virtual machine instance (`VmId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VmId(Uuid);

impl std::fmt::Display for VmId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for VmId {
    type Error = uuid::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for VmId {
    fn default() -> Self {
        Self::new()
    }
}

impl VmId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

/// Lifecycle state of a virtual machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    /// VM definition exists but is not running.
    Defined,
    /// VM is currently booting or resuming.
    Starting,
    /// VM is running normally.
    Running,
    /// VM has been paused by user request.
    Paused,
    /// VM is shutting down gracefully.
    Stopping,
    /// VM has stopped (clean shutdown or power-off).
    Stopped,
    /// VM encountered an error condition.
    Error(String),
}

/// Specification for a virtual machine instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSpec {
    pub name: String,
    pub id: Option<VmId>,
    /// Number of virtual CPUs to allocate.
    pub vcpus: u32,
    /// Memory in MiB.
    pub memory_mb: u64,
    /// Path to disk image or libvirt storage pool reference.
    pub disk_image: String,
    /// Boot order: 0=first, higher numbers tried later.
    pub boot_order: Option<u8>,
}

/// Event emitted during VM lifecycle transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub vm_id: VmId,
    pub event_type: EventType,
    pub timestamp: u64, // Unix epoch seconds
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Created,
    Starting,
    Started,
    Paused,
    Resumed,
    Stopping,
    Stopped,
    SnapshotCreated(String), // snapshot name
    SnapshotRestored(String),
    ErrorOccurred,
}

/// Request to create a VM snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotRequest {
    pub vm_id: VmId,
    pub name: String,
    pub description: Option<String>,
}

/// Response from snapshot operations.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotInfo {
    pub vm_id: VmId,
    pub name: String,
    pub created_at: u64,
    pub disk_size_bytes: u64,
}

/// Policy metadata attached to VM lifecycle actions for guard evaluation.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub actor: String,
    pub action: &'static str,
    pub resource: String,
}