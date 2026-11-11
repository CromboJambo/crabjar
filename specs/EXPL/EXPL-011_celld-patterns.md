# EXPL-011: Steal celld's Patterns into crabjar (Native Rust Coordination)

**Status**: Proposed — Experiment Branch  
**Branch**: `exp/celld-patterns`  
**Date**: 2026-11-10  
**Motivation**: celld solved distributed coordination with an elegant, minimal design: object storage as the only external dependency, CAS ownership with fencing epochs, LTX replication for RPO=0. crabjar currently has ad-hoc fleet coordination (conductor + supervisor scripts) that lacks these guarantees. This experiment ports celld's *patterns* into crabjar's Rust codebase directly — no JS boundary, no Workers model, just the coordination primitives.

## Hypothesis

crabjar can adopt celld's bucket-ownership model and LTX replication as native Rust crates, giving workers durable state and automatic failover without depending on celld as a runtime. The patterns are language-agnostic; only the implementation (deno_core/V8/JS) is specific to celld.

## What Success Looks Like

1. A new `crabjar-coordination` crate implements:
   - Bucket-based ownership claims (CAS on `cells/<name>/own.json`)
   - Fencing epochs that invalidate stale owners
   - Node lease management (`nodes/<id>.json` with TTL)
2. Worker state (currently in conductor's SQLite) is replicated via LTX-style WAL streaming to a secondary node + object storage
3. Kill the primary worker node → secondary takes over within one lease lifetime with zero data loss
4. crabjar retains full Rust-native types throughout (no JSON serialization of trust scores, provenance IDs, JEV judgments)

## What Would Kill It

- The coordination logic is too entangled with celld's actor model to extract cleanly — you can't have bucket ownership without named cells, and named cells imply the Workers programming model
- LTX replication requires SQLite WAL format compatibility that's harder to implement correctly than it looks (block checksums, sequence numbers, truncation)
- crabjar's current conductor design (centralized task queue) is fundamentally at odds with celld's decentralized ownership model — you'd have to redesign the conductor, not just add a crate
- The effort to port is 3-6 months of work for something that works "good enough" today

## What We're Stealing (and what we're not)

**Steal:**
- Bucket as coordinator: no etcd, no Raft, no membership protocol. S3-compatible object storage with conditional writes is the consensus mechanism.
- CAS ownership: `cells/<name>/own.json` with `If-None-Match` / `If-Match` headers. First writer wins; stale writers get 409 and self-fence.
- Fencing epochs: every ownership claim increments an epoch number. Stale owners presenting old epochs are rejected by followers and the bucket.
- LTX replication: stream SQLite WAL segments to object storage + peer nodes. On failover, restore from local follower disk + bucket chain.
- Self-fencing on lease loss: node detects it's no longer the owner, stops serving, exits with code 3. Supervisor restarts it; cold activation recovers state.
- The "ensemble needs two nodes" rule: one node = bucket-proof latency (correct but slow). Two nodes = follower fsync (fast + durable). Be honest about what posture you have.

**Don't steal:**
- The Workers/Durable Objects programming model (JS/TS, fetch handlers, WebSockets) — crabjar is Rust-native
- deno_core/V8 isolation — crabjar workers are Rust processes/threads
- The single-binary deployment model — crabjar is already a workspace with multiple binaries
- Cloudflare compatibility layer — we're self-hosting, not running CF Workers

## First Concrete Step

1. Stand up MinIO on jambo (if not already running)
2. Write a 200-line Rust prototype: two processes competing for ownership of a "cell" via S3 conditional writes
3. Verify: process A owns the cell, kill it, process B acquires it within lease lifetime
4. Add fencing epoch: restart process A, verify it's rejected as stale
5. If this works cleanly → design the `crabjar-coordination` crate API

## Architecture Sketch

```
┌──────────────────────────────────────────────────────┐
│              crabjar-conductor (existing)            │
│  task queue, worker registry, goal tracking          │
└──────────────────────┬───────────────────────────────┘
                       │ assigns work to workers
                       ▼
┌──────────────────────────────────────────────────────┐
│           crabjar-coordination (NEW crate)           │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │ Ownership   │  │ Fencing      │  │ Node Leases│  │
│  │ (CAS + S3)  │  │ Epochs       │  │ (TTL + S3) │  │
│  └─────────────┘  └──────────────┘  └────────────┘  │
│  ┌─────────────────────────────────────────────┐    │
│  │ LTX Replication (WAL → S3 + peer nodes)     │    │
│  └─────────────────────────────────────────────┘    │
└──────────────────────┬───────────────────────────────┘
                       │ durable state for
                       ▼
┌──────────────────────────────────────────────────────┐
│              crabjar workers (existing)              │
│  guard decisions, JEV judgments, tool execution      │
│  state in SQLite, replicated via coordination crate  │
└──────────────────────────────────────────────────────┘

Bucket (MinIO): ownership records + node leases + LTX stream
```

## Open Questions

- Does crabjar need per-worker cells, or is one cell per "goal" sufficient? (celld: one cell per DO instance. crabjar: ?)
- How does the conductor's centralized task queue interact with decentralized ownership? (Conductor says "worker-42, do this goal." Coordination crate says "worker-42 is dead, worker-7 has its state.")
- LTX replication to object storage is async — what's the RPO window if both nodes die before upload completes? (celld: bucket proof fallback. crabjar: ?)
- Can we use rusqlite's existing WAL hooks, or do we need to implement the LTX block format from scratch?

## Related

- EXPL-010 (run on celld) — alternative if this is too much work
- EXPL-012 (hybrid) — if both work, use patterns for core + celld for edge workers
- celld `crates/logic/` (~13K LOC) — the pure decision core we're modeling after
- celld `docs/guarantees.md` — the formal guarantees we're trying to match
