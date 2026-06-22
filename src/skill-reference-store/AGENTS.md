# AGENTS.md — skill-reference-store (skill-reference-store)

> Purpose: Skill reference indexing and staleness detection — keep skill references fresh.

## Layer

Layer 7: skills — agent skills, may depend on all layers.

## Public API

- Reference indexing (scan and index skill references)
- Staleness detection (compare indexed refs against filesystem)
- Reference validation (verify referenced files exist)

## Key Files

- `src/lib.rs` — crate entry point
- `src/indexer.rs` — reference indexing logic
- `src/staleness.rs` — staleness detection logic

## Dependencies

- anyhow, serde, serde_json, rusqlite, chrono, uuid, path-absolutize, thiserror, tempfile

## Pitfalls

- Staleness detection compares checksums against indexed state
- Indexing should be incremental to avoid full rescan on every check
- References to non-existent files should be flagged as stale
