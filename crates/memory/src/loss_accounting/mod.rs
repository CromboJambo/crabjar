//! Loss Accounting — Auditable Context Compression.
//!
//! Every compression operation produces a `LossProfile` that records what was
//! compressed, how it was compressed, what we think was lost (with confidence),
//! and where the original can be found. This turns compression from a black box
//! into an auditable cost center.
//!
//! Core principle: **Unknown is a valid value.** If you don't know what was
//! lost, say `Unknown`. Don't invent precision you don't have.

pub mod audit;
pub mod models;
pub mod policies;

#[cfg(test)]
mod tests;

pub use audit::{ApproverRef, CompressionAuditEntry, CompressionAuditLog};
pub use models::{
    AccessCost, CompressionStrategy, Confidence, LossEstimate, LossProfile, LossProfileId,
    Recoverability, RecoveryRef, SourceRef, Timestamp, TruncationSide,
};
pub use policies::{
    ArtifactType, CompressionPolicy, ConfidenceLevel, FidelityProfile, MeasurementBasis, TaskType,
    confidence_meets_requirement,
};
