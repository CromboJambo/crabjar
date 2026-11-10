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

pub mod api;
pub mod attention;
pub mod audit;
pub mod auth;
pub mod conductor;
pub mod conductor_gate;
pub mod decision;
pub mod fleet_status;
pub mod gate;
pub mod promotion;
pub mod sandbox;
pub mod state_store;
pub mod task_scheduler;
pub mod task_spec;
pub mod worker_registry;

// Re-export core types for ergonomic use
pub use conductor::{Conductor, ConductorConfig};
pub use promotion::PromotionManager;
pub use state_store::Worker;
pub use state_store::{Goal, StateStore, Task};
pub use task_scheduler::TaskScheduler;
pub use task_spec::TaskSpec;
pub use worker_registry::WorkerRegistry;
