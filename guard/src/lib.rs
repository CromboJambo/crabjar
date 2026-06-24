//! mirror-guard: Trust-layer gating and execution authorization.
//!
//! Provides the action-gating layer for the mirror-lab workspace, maintaining
//! strict separation between detection (mirror-log) and authorization (mirror-guard).
//!
//! ## Core Components
//!
//! - **Trust Layers** (`trust`): Confidence bands that determine auto-execute behavior
//! - **Execution Gate** (`gate`): The single point where detection becomes authorized action
//! - **Concierge** (`concierge`): Pending queue management and provenance verification
//!
//! ## Architecture
//!
//! ```text
//! mirror-log (detection)  ──events──>  mirror-guard (authorization)  ──gated──>  mirror-daemon (action)
//!     append-only                  separate DB (guard.db)                  execution gate
//! ```
//!
//! ## Key Principles
//!
//! - **Detection != Authorization**: Knowing what happened doesn't grant the right to act
//! - **Every Abstraction Carries Doubt**: Outputs include uncertainty, assumptions, and staleness info

pub mod concierge;
pub mod fingerprint;
pub mod gate;
pub mod guard_db;
#[cfg(test)]
pub mod memory;
pub mod scope;
pub mod trust;
pub mod trust_resolution;
pub mod types;

pub use concierge::{GateConcierge, InterruptedLogEntry, PendingQueueEntry};
pub use fingerprint::{ApprovalLease, ApprovalScope, InMemoryApprovalStore, InvocationFingerprint};
pub use gate::{CommandRisk, ExecutionGate, GateContext, GateResult};
pub use guard_db::{GuardDb, GuardDbError};
#[cfg(test)]
pub use memory::MemoryGraph;
pub use scope::{
    CrossScopeAuth, Identity, ProjectId, Scope, ScopeError, ScopedAccess, TenantId, ThreadId,
};
pub use trust::TrustManager;
pub use trust_resolution::{
    EffectiveTrust, Policy, PolicyChain, PolicySource, RequestedTrust, TrustResolution,
    TrustResolver,
};
pub use types::*;
