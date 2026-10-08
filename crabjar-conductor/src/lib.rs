//! Crabjar Conductor — Fleet orchestration service for crabjar agent coordination.
//!
//! Provides centralized goal management, task decomposition, worker registration,
//! and cross-machine scheduling for home-lab agent fleets.
//!
//! # Architecture
//!
//! ```hemlock
//! User → Conductor Agent (ftw3) → Worker Agents (jambo, other nodes)
//!          ↑ HTTP API            ↑ SSH + crabjar daemon
//!          │                      │
//!     Task queue                Execute tasks
//!     State store               Report results
//!     Capability registry       Request more work
//! ```
//!
//! # Core Types
//!
//! - [`Goal`] — High-level objective submitted by a user
//! - [`Task`] — Executable unit of work with dependency tracking
//! - [`Worker`] — Registered fleet member with capability metadata

pub mod conductor;
pub mod worker_registry;
pub mod task_scheduler;
pub mod state_store;
pub mod api;

// Re-export core types for ergonomic use
pub use conductor::{Conductor, ConductorConfig};
pub use worker_registry::WorkerRegistry;
pub use state_store::Worker;
pub use task_scheduler::TaskScheduler;
pub use state_store::{StateStore, Goal, Task};
