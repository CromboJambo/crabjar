# AGENTS.md — crabjar-guard (guard)

> Purpose: Execution gate — trust layers, annealing, and authorization for crabjar's agent execution pipeline.

## Layer

Layer 1: substrate — low-level storage, may depend on layer 0 only.

## Public API

- `ExecutionGate` — single authorization boundary for all tool execution
- `GateConcierge` — enforce deny/pending/proceed decisions, construct handoff envelopes
- `TrustManager` — confidence bands and trust evolution
- `AnnealingPipeline` — confidence decay and reinforcement
- `RetrievalEngine` — layer-based querying for trust evaluation
- `MemoryGraph` — trust memory nodes + edges
- `ReversibilityScore` — perturbation set for reversible actions
- `GuardDb` — SQLite-backed pending actions, outcomes, transition audit log
- `JevArbiter` — belief-revision decision engine for queued actions
- `HandoffEnvelope` — self-contained context bundle for cross-process action handoffs

## Key Files

- `src/lib.rs` — crate entry point
- `src/gate/` — ExecutionGate (single authorization boundary)
- `src/concierge.rs` — GateConcierge (enforce: deny/pending/proceed, envelope construction)
- `src/trust.rs` — TrustManager (confidence bands)
- `src/annealing.rs` — AnnealingPipeline (confidence decay/reinforcement)
- `src/retrieval.rs` — RetrievalEngine (layer-based querying)
- `src/memory.rs` — MemoryGraph (nodes + edges)
- `src/reversibility.rs` — ReversibilityScore → PerturbationSet
- `src/arbiter/` — JEV Arbiter (belief-revision decision engine for queued actions)
- `src/guard_db.rs` — GuardDb (open, schema init, rehydration validation)
- `src/guard_db_impl.rs` — GuardDb core persistence (action requests, pending queue, interrupted log)
- `src/guard_db_queries.rs` — GuardDb query helpers + validated rehydration reads
- `src/guard_db_handoffs.rs` — Handoff envelope persistence + transition logging
- `src/guard_db_types.rs` — Persistence types (ActionTransition, TransitionError, validate_transition)
- `src/handoff.rs` — HandoffEnvelope contract types (Phase 2)
- `src/schema.sql` — GuardDb schema definition
- `src/types.rs` — domain types

## Persistence Model

GuardDb enforces state consistency through:

1. **Transition audit log** (`action_transitions` table) — every status change records from/to/authority/reason
2. **Rehydration validation** — on startup, validates all action requests against their transition logs; flags inconsistencies
3. **Handoff envelopes** (`handoffs` table) — idempotent envelope storage linked to action IDs
4. **Authority separation** — transitions require an authority string (e.g., "concierge", "user:cli")

Schema version is tracked in `schema_meta` and verified at open time. Inconsistent rows are logged as warnings but do not block startup (soft failure mode).

## Dependencies

- tokio, serde, serde_json, thiserror, rusqlite, chrono, uuid, tracing, tracing-subscriber, tempfile

## Pitfalls

- Parameter names must match column names exactly (semantic naming drift causes structural bugs)
- Pending actions persist to GuardDb — never lose them
- The guard is the sole gate layer — no duplicate enforcement
- `crabjar guard` CLI commands delegate to this crate
- Annealing tracks trust evolution across the action pipeline
