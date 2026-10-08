//! Unit tests for vm-core domain types.

use vm_core::types::*;

#[test]
fn vm_id_generates_unique_values() {
    let id1 = VmId::new();
    let id2 = VmId::new();
    assert_ne!(id1, id2, "VmId instances should be unique");
}

#[test]
fn vm_state_serialization() {
    let state = VmState::Running;
    let json = serde_json::to_string(&state).unwrap();
    let restored: VmState = serde_json::from_str(&json).unwrap();
    assert!(matches!(restored, VmState::Running));
}

#[test]
fn vm_spec_roundtrip() {
    let spec = VmSpec {
        name: "test-vm".to_string(),
        id: None,
        vcpus: 2,
        memory_mb: 1024,
        disk_image: "/var/lib/libvirt/images/test.qcow2".to_string(),
        boot_order: Some(0),
    };

    let json = serde_json::to_string(&spec).unwrap();
    let restored: VmSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.name, "test-vm");
    assert_eq!(restored.vcpus, 2);
}

#[test]
fn snapshot_request_creation() {
    let vm_id = VmId::new();
    let req = SnapshotRequest {
        vm_id,
        name: "pre-update".to_string(),
        description: Some("Before system update".to_string()),
    };
    assert_eq!(req.name, "pre-update");
}

#[test]
fn lifecycle_event_creation() {
    let vm_id = VmId::new();
    let event = LifecycleEvent {
        vm_id,
        event_type: EventType::Created,
        timestamp: 1728403200, // Fixed for determinism
        detail: None,
    };
    assert!(matches!(event.event_type, EventType::Created));
}
