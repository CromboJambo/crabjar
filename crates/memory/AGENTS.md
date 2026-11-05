# AGENTS.md — agent-context (memory)

> Purpose: Agent-context SQLite storage for crabjar's knowledge and state-docs.

## Layer

Layer 1: substrate — low-level storage, may depend on layer 0 only.

## Public API

- `StateDocsQuerier` — query state-docs from SQLite
- `drift_status()` — compare state-docs against filesystem reality
- `KnowledgeStore` — insert/query/deactivate knowledge entries
- `EventStore` — append-only event log

## Key Files

- `src/lib.rs` — crate entry point
- `src/state_docs/indexer.rs` — state-doc indexing
- `src/state_docs/querier.rs` — state-doc querying + drift detection
- `src/state_docs/renderer.rs` — annotation rendering
- `src/state_docs/models.rs` — state-doc domain models
- `src/state_docs/schema.rs` — SQLite schema for state-docs
- `src/schema.rs` — general database schema
- `src/models.rs` — knowledge domain models
- `src/error.rs` — error types
- `tests/state_docs_tests.rs` — integration tests

## Dependencies

- tokio, serde, serde_json, thiserror, rusqlite, chrono, tempfile

## Pitfalls

- SQLite uses `:memory:` or tempfile paths — never write to the repository
- State-docs are append-only — never delete, only mark as resolved/deactivated
- Drift detection compares checksums of state-doc content against filesystem
- `crabjar knowledge` CLI commands delegate to this crate
