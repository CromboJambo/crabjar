# EXPL-010: Run crabjar Workers on celld (Celld as Execution Substrate)

**Status**: Proposed — Experiment Branch  
**Branch**: `exp/celld-substrate`  
**Date**: 2026-11-10  
**Motivation**: celld provides durable, failover-capable execution of named actors (Durable Objects) with SQLite state, using only object storage as external dependency. crabjar workers need exactly this: named, stateful, recoverable execution units on home-lab hardware.

## Hypothesis

crabjar agent workers can run as celld Durable Objects, gaining automatic failover, durable state (RPO=0), and hibernation/recovery — without crabjar implementing its own coordination layer. The JS/TS boundary is acceptable because worker logic is thin orchestration over HTTP calls to crabjar's Rust core.

## What Success Looks Like

1. A crabjar worker (e.g., a code-review agent) runs as a Durable Object on a 2-node celld fleet (jambo + ftw3)
2. Kill the node hosting the worker → it recovers on the other node within lease lifetime (~10s) with zero state loss
3. Multi-step tasks survive node death mid-execution (using celld's `step.do()` workflow primitive)
4. Worker state (session history, tool results, intermediate findings) persists in the DO's SQLite and survives restarts

## What Would Kill It

- The JS/TS boundary is too lossy for crabjar's Rust-native types (provenance IDs, trust scores, JEV judgments) — you can't round-trip them through JSON without losing semantic structure
- celld's single-threaded actor model is too slow for I/O-heavy agent workloads (many concurrent HTTP calls to local LLM servers)
- The Workers programming model (request/response fetch handler) doesn't fit long-running agent sessions well enough, even with WebSockets
- v0.6.3 beta instability blocks real usage

## First Concrete Step

Deploy a 2-node celld fleet on jambo + ftw3:
1. Install MinIO on one node (or use existing S3-compatible storage)
2. `curl -fsSL https://celld.dev/install.sh | sh` on both nodes
3. Create dinit service files with unlimited respawn and 10s delay
4. Deploy the celld `counter` example, verify failover by killing one node
5. Deploy a minimal "echo agent" Worker that calls a local llama-server via HTTP

## Architecture Sketch

```
┌─────────────────────────────────────────────────────────┐
│                    celld Fleet                          │
│  ┌──────────────┐         ┌──────────────┐              │
│  │   jambo      │◄────────►│    ftw3      │              │
│  │  (celld node)│  LTX     │  (celld node)│              │
│  │  DO: worker-1│         │  DO: worker-2│              │
│  └──────┬───────┘         └──────┬───────┘              │
│         │                        │                       │
└─────────┼────────────────────────┼───────────────────────┘
          │ HTTP                   │ HTTP
          ▼                        ▼
   ┌─────────────┐          ┌─────────────┐
   │ llama-server│          │ crabjar core│ (guard, trust, JEV)
   │ :8081       │          │ :8091       │
   └─────────────┘          └─────────────┘

Bucket (MinIO): ownership records + LTX replication stream
```

Each worker DO:
- Receives tasks via HTTP POST (from conductor or human)
- Calls crabjar core for guard decisions, trust checks, JEV judgments
- Calls local llama-server for inference
- Persists session state in its own SQLite (managed by celld)
- Survives node death via LTX replication + bucket ownership

## Open Questions

- How does the conductor assign work to specific DOs? (Named cells = named workers, so `env.Workers.get("worker-42")` is the lookup)
- Can we expose crabjar's Rust core as a WASM module that runs *inside* the DO, eliminating the HTTP boundary? (celld has a wasm example)
- What's the latency profile of a DO making 50 sequential HTTP calls to localhost vs. running the logic natively in Rust?

## Related

- EXPL-011 (steal patterns into crabjar) — alternative if this fails
- EXPL-012 (hybrid) — if both work, combine them
- celld `examples/opencode/` — proof that AI agents run in DOs
- celld `examples/sandbox/` — proof that command execution runs in DOs
