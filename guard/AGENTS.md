# AGENTS.md — crabjar-guard (guard)

> Purpose: Execution gate — trust layers, annealing, and authorization for crabjar's agent execution pipeline.

## Layer

Layer 1: substrate — low-level storage, may depend on layer 0 only.

## Public API

- `ExecutionGate` — single authorization boundary for all tool execution
- `GateConcierge` — enforce deny/pending/proceed decisions
- `TrustManager` — confidence bands and trust evolution
- `AnnealingPipeline` — confidence decay and reinforcement
- `RetrievalEngine` — layer-based querying for trust evaluation
- `MemoryGraph` — trust memory nodes + edges
- `ReversibilityScore` — perturbation set for reversible actions
- `GuardDb` — SQLite-backed pending actions and outcomes

## Key Files

- `src/lib.rs` — crate entry point
- `src/gate.rs` — ExecutionGate (single authorization boundary)
- `src/concierge.rs` — GateConcierge (enforce: deny/pending/proceed)
- `src/trust.rs` — TrustManager (confidence bands)
- `src/annealing.rs` — AnnealingPipeline (confidence decay/reinforcement)
- `src/retrieval.rs` — RetrievalEngine (layer-based querying)
- `src/memory.rs` — MemoryGraph (nodes + edges)
- `src/reversibility.rs` — ReversibilityScore → PerturbationSet
- `src/guard_db.rs` — GuardDb (SQLite schema + queries)
- `src/schema.sql` — GuardDb schema definition
- `src/types.rs` — domain types

## Dependencies

- tokio, serde, serde_json, thiserror, rusqlite, chrono, uuid, tracing, tracing-subscriber, tempfile

## Pitfalls

- Parameter names must match column names exactly (semantic naming drift causes structural bugs)
- Pending actions persist to GuardDb — never lose them
- The guard is the sole gate layer — no duplicate enforcement
- `crabjar guard` CLI commands delegate to this crate
- Annealing tracks trust evolution across the action pipeline
