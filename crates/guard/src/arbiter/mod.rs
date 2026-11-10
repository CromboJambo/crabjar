//! JEV Arbiter — belief-revision decision engine for queued actions.
//!
//! When `GateConcierge::enforce()` produces a `Queued` outcome, the arbiter
//! takes over and runs multiple evaluation passes, accumulating confidence
//! until it either auto-approves, auto-denies, or escalates to human review
//! with a structured excuse explaining why it couldn't decide.
//!
//! # Design Principles
//! - **Implicit assumptions**: Each pass encodes an assumption about what makes
//!   an action safe/risky. The aggregate decides — no single pass needs full criteria.
//! - **No throttling**: Fast passes don't wait for slow ones (async at orchestrator layer).
//! - **Structured excuses**: When the arbiter can't decide, it explains why.
//! - **Typestate safety**: Operates on `GuardedAction<Pending>`, produces only valid transitions.

#[allow(clippy::module_inception)]
pub mod arbiter;
pub mod pass;
pub mod types;

// Re-exports for ergonomic use
pub use arbiter::JevArbiter;
pub use pass::{
    CommandRiskPass, ConfidencePass, EvaluationPass, HistoryPass, PassError, ProvenancePass,
    TrustLayerPass,
};
pub use types::{
    ArbiterState, ArbitrationExcuse, ArbitrationRecord, ArbitrationResult, ArbitrationStep,
    Evidence,
};
