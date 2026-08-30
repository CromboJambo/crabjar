//! Exact-invocation fingerprint types — prevents approval smuggling.
//!
//! IronClaw's `ironclaw_approvals` uses SHA-256 fingerprints of exact tool
//! invocations (command + args + context) for approval decisions. Crabjar
//! adopts this pattern to close the gap where pattern-based approvals create
//! a false sense of security.

pub use crate::fingerprint_types::{
    ApprovalLease, ApprovalScope, InMemoryApprovalStore, InvocationFingerprint,
};
