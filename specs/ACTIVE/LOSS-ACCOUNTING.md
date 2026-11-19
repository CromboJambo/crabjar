# Loss Accounting — Auditable Context Compression

**Date:** November 19, 2026
**Status:** Draft
**Priority:** P2 (Productivity Improvement)
**North Star:** Compression is a cost. Track what was lost so you know when to stop compressing. Unknown is a valid value — don't invent precision.

## Problem Statement

Crabjar compresses worker context every turn to stay within token budgets. Currently, this compression is opaque — we know the size went down, but not what semantic information was lost or how much fidelity remains for different types of queries. A 3-fold compressed context might be perfect for architecture questions and useless for debugging. We need to make compression auditable.

## Core Concept

Every compression operation produces a **LossProfile** that records:
- What was compressed (size, type, content class)
- How it was compressed (strategy, depth)
- What we think was lost (estimated loss, with confidence)
- Where the original can be found (recovery path)

This turns compression from a black box into an auditable cost center. Like your day job: you don't just say "we saved money" — you say what was cut and what the risk is.

## Architecture

### LossProfile Data Model

```rust
pub struct LossProfile {
    pub id: LossProfileId,
    pub created_at: Timestamp,
    
    // What was compressed
    pub source: SourceRef,              // Original artifact/context
    pub original_size: usize,           // Bytes or tokens
    pub target_size: Option<usize>,     // Desired size (if any)
    pub final_size: usize,              // Actual result
    
    // How it was compressed
    pub strategy: CompressionStrategy,
    pub reduction_depth: u32,           // Number of folds applied
    pub reduction_ratio: f32,           // original / final
    
    // What we think was lost
    pub estimated_loss: LossEstimate,
    pub loss_confidence: Confidence,    // How sure are we about the estimate?
    
    // Recovery
    pub recovery_path: Option<RecoveryRef>,  // Where to find the original
    pub recoverability: Recoverability,      // Can we actually get it back?
}

pub enum CompressionStrategy {
    Summarization { model: String },    // LLM-based summary
    Truncation { from: TruncationSide },// Keep head/tail
    Sampling { rate: f32 },             // Random or stratified sampling
    Abstraction { level: u32 },         // Generalize specifics
    Aggregation { group_by: String },   // Combine similar items
}

pub enum LossEstimate {
    None,                    // Lossless (verified)
    Bounded { max_items: usize, description: String },
    Estimated { probability: f32, description: String },
    Unknown { reason: String },         // Valid! Don't guess.
}

pub enum Confidence {
    Verified,                // Measured against ground truth
    High { basis: String },  // Strong evidence
    Medium { basis: String },// Reasonable assumption
    Low { basis: String },   // Guess
    Unknown,                 // No basis for estimate
}

pub struct RecoveryRef {
    pub source_id: SourceId,           // Original artifact ID
    pub storage_location: StoragePath, // Where it's stored
    pub access_cost: AccessCost,       // How expensive is recovery?
    pub freshness_verified: bool,      // Is the original still valid?
}

pub enum Recoverability {
    Lossless,                    // Full source accessible
    Bounded { max_depth: u32 },  // Can traverse N levels back
    ReferenceOnly,               // Has pointers but not full content
    Unknown,                     // Don't know yet
}
```

### Compression Decision Policy

Different artifact types have different loss policies:

| Artifact Type | Loss Policy | Max Depth | Recovery Required |
|---------------|-------------|-----------|-------------------|
| Raw observations | Lossless | 0 | N/A (never compressed) |
| Worker scratchpad | Bounded loss | 2 | Reference only |
| Context summary | Measured reduction | 3 | Full recovery |
| Task history | Aggregation | 4 | Bounded recovery |
| Institutional memory | Minimal loss | 1 | Lossless |

```rust
pub struct CompressionPolicy {
    pub artifact_type: ArtifactType,
    pub max_reduction_depth: u32,
    pub required_recoverability: Recoverability,
    pub min_fidelity_estimate: f32,      // Don't compress below this confidence
    pub loss_confidence_required: ConfidenceLevel,  // How sure must we be about the loss?
}
```

### Compression Audit Log

Every compression operation is logged:

```rust
pub struct CompressionAuditEntry {
    pub timestamp: Timestamp,
    pub worker_id: WorkerId,
    pub source_ref: SourceRef,
    pub profile_id: LossProfileId,
    pub decision_rationale: String,      // Why compress? Why this depth?
    pub approved_by: Option<ApproverRef>,// Did a supervisor approve deep compression?
}
```

### Fidelity Estimation

Fidelity is estimated per task type, not globally:

```rust
pub struct FidelityProfile {
    pub task_type: TaskType,
    pub fold_depth: u32,
    pub estimated_fidelity: f32,         // P(correct answer | compressed context)
    pub measurement_basis: MeasurementBasis,
}

pub enum MeasurementBasis {
    Empirical { samples: usize },        // Measured on real tasks
    Analogical { similar_task: TaskType },// Estimated from similar task type
    Theoretical { model: String },       // Based on information theory bounds
    Unknown,                             // No basis — use conservative default
}
```

Workers operating on compressed context should check the fidelity profile for their task type before proceeding. If estimated fidelity is below threshold, they request deeper context or a different compression strategy.

## Integration Points

### With Waypoint System
Compressed waypoints carry LossProfile references. Workers checking `recoverability` are actually reading the LossProfile to understand what they're working with.

### With Lineage Provenance
Compression decisions are recorded as lineage nodes. If a worker later discovers that important information was lost in compression, it can trace back to the compression decision and flag it for review.

### With Observer Workers
Observers can analyze compression patterns across workers: "Worker X's tasks consistently fail at fold depth 3 — maybe we should reduce compression for their task type."

## Constraints & Policies

### Unknown Is Valid
If you don't know what was lost, say `Unknown`. Don't invent a precision you don't have. An honest "we don't know what this compression lost" is better than a fabricated "estimated 5% loss."

### Depth Limits Are Hard
Workers cannot exceed their assigned compression depth without supervisor approval. This prevents runaway compression where context gets folded so many times it's unrecognizable.

### Recovery Must Work
If a LossProfile claims recovery is possible, the recovery path must be tested periodically. A recovery reference that points to deleted data is worse than no reference at all (it creates false confidence).

## Success Criteria

1. Every compressed context has an associated LossProfile
2. Workers can check fidelity estimates before relying on compressed context
3. Compression decisions are auditable — we know why, how deep, and what was estimated lost
4. Recovery paths work when tested (quarterly verification)

## Open Questions

- How do we measure fidelity without ground truth? (A/B testing with full context?)
- Should workers be able to request different compression strategies for the same artifact?
- What's the right balance between compression depth and recovery cost?
