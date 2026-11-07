---
id: ADR-009
title: Hybrid Attention + Semantic Decision Layer
status: Accepted
date: 2026-10-18
supersedes: null
see_also: [[ADR-007_earned_autonomy]]
---

# ADR-009: Hybrid Attention + Semantic Decision Layer

## Status

Accepted — implemented and tested in crabjar-conductor v0.4.0.

## Context

Crabjar's JEV (Just Enough Verification) decision gate uses embedding-based semantic evaluation for every observation, which works well at small scale but has latency and throughput limitations:

- Single-stage JEV evaluation: ~679µs average per observation
- Throughput ceiling: ~1,000 observations/sec
- Embedding API calls: 100% of observations (expensive at fleet scale)

Mirror-lab's attention-based observation triage provides fast rule-based scoring using temporal decay and access frequency (~135µs per observation), but lacks semantic understanding.

We needed a fleet-scale solution that combines the speed of attention-based filtering with the accuracy of JEV semantic evaluation, reducing embedding API calls while maintaining decision quality for high-priority items.

## Decision

Implement a two-stage hybrid pipeline in crabjar-conductor:

1. **Stage 1 — Attention Triage**: Fast rule-based scoring using temporal decay, source type weights, and content length heuristics. Filters ~90% of observations (low scores → Ignore).
2. **Stage 2 — JEV Semantic Evaluation**: Only high-attention items (>70 score threshold) escalate to embedding-based decision gate for deep semantic matching against criteria files.

The hybrid layer wraps the existing `DecisionGate` and is configured via optional `mirror_log_db_path` in `ConductorConfig`. When no attention database is provided, conductor operates in JEV-only mode (backward compatible).

## Options Considered

- **JEV-only (status quo)**: Simple architecture but doesn't scale to fleet throughput requirements.
- **Attention-only**: Fast but lacks semantic understanding; would miss contextually important items with low temporal scores.
- **Separate services for attention and JEV**: More complex deployment, network overhead between stages. Chose in-process integration instead.
- **Hard-coded thresholds**: Less flexible than configurable per-fleet-profile thresholds.

## Consequences

### Positive consequences

- 4.5x faster average latency (~150µs vs 679µs)
- 5x higher throughput (5,000+ obs/sec vs 1,000)
- 90% reduction in embedding API calls
- Configurable thresholds per fleet profile via `decision_policy.json`
- Graceful degradation if JEV evaluation fails (falls back to attention-only decision)

### Negative consequences (trade-offs)

- More complex architecture with two decision layers
- Requires threshold tuning per fleet profile
- Additional SQLite connection management for attention database
- Slight increase in memory footprint due to hybrid layer state

### Ongoing concerns

- Threshold values (low=30, high=70) are defaults; may need tuning based on actual fleet observation distributions
- Long-term monitoring of JEV escalation rate to ensure it stays within expected ~10% range
- Periodic review of attention scoring heuristics as observation types evolve

## References

- `specs/hybrid-attention-decision.md` — original spec (archived)
- `/home/crombo/projects/active/mirror-lab/mirror-log/src/attention/mod.rs` — reference attention implementation
- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang