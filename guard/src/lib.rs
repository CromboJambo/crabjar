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

pub mod action;
pub mod command_risk;
pub mod concierge;
pub mod concierge_types;
pub mod fingerprint;
pub mod fingerprint_types;
pub mod gate;
pub mod gate_context;
pub mod gate_result;
#[cfg(test)]
pub mod gate_tests;
pub mod guard_db;
pub mod guard_db_impl;
pub mod inference;
pub mod memory;
pub mod memory_types;
pub mod risk_config;
pub mod scope;
pub mod trust;
pub mod trust_types;
pub mod trust_resolution;

// Re-export types from split modules for backward compatibility
pub use action::{ActionOutcome, ActionRequest, ActionStatus, OutcomeStatus};
pub use command_risk::{CommandRisk, HIGH_RISK_COMMANDS, MEDIUM_RISK_COMMANDS};
pub use concierge::{GateConcierge, InterruptedLogEntry, PendingQueueEntry};
pub use fingerprint::{ApprovalLease, ApprovalScope, InMemoryApprovalStore, InvocationFingerprint};
pub use gate::ExecutionGate;
pub use gate_context::GateContext;
pub use gate_result::GateResult;
pub use guard_db::{GuardDb, GuardDbError};
pub use guard_db_impl::TrustResolutionEntry;
pub use memory::MemoryGraph;
pub use memory_types::{EdgeRelation, MemoryEdge, MemoryNode, NodeKind};
pub use risk_config::RiskConfig;
pub use scope::{
    CrossScopeAuth, Identity, ProjectId, Scope, ScopeError, ScopedAccess, TenantId, ThreadId,
};
pub use trust::{
    AnnealConfig, AnnealResult, PidTrustRecord, RevokedLogEntry, RetrievalBand, ReviewAction,
    ReviewRecord, TrustLayer, TrustManager, TrustScore,
};
pub use trust_resolution::{
    EffectiveTrust, Policy, PolicyChain, PolicySource, RequestedTrust, TrustResolution,
    TrustResolver,
};
pub use inference::{ModelInferenceKind, ModelInferenceOutcome, ModelInferenceRequest};
