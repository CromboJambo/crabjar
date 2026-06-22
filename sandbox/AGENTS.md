# AGENTS.md — crabjar-sandbox (sandbox)

> Purpose: Agent isolation — Unix user sandboxing, container isolation, and cgroup management.

## Layer

Layer 1: substrate — low-level storage, may depend on layer 0 only.

## Public API

- `AgentIsolation` — Unix user sandbox, container isolation, cgroup management
- Schema for sandbox state persistence

## Key Files

- `src/lib.rs` — crate entry point
- `src/agent_isolation.rs` — isolation logic
- `src/schema.rs` — SQLite schema
- `src/error.rs` — error types

## Dependencies

- tokio, serde, serde_json, chrono, uuid, rusqlite, tracing, thiserror, path-absolutize, tempfile

## Pitfalls

- Sandbox isolation is a security boundary — verify container/cgroup paths
- Unix user sandboxing requires root — never execute sudo commands (present as user-run)
- Detection ≠ authorization: observer reports must not trigger execution
- Reversibility gating: destructive actions require user permission
