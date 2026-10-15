//! vm-core: VM lifecycle management substrate for crabjar habitat.
//!
//! Provides typed APIs for starting, stopping, snapshotting, and restoring
//! virtual machines via libvirt, integrated with crabjar-guard policy enforcement.
//!
//! Architecture:
//! - `types` — domain types (`VmSpec`, `VmState`, `LifecycleEvent`)
//! - `backend` — libvirt/libguestfs FFI bindings
//! - `manager` — high-level `VMManager` with guard integration
//! - `error` — typed error hierarchy

pub mod backend;
pub mod error;
pub mod manager;
pub mod types;

pub use backend::*;
pub use error::VmError;
pub use manager::*;
pub use types::*;
