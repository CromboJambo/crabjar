# AGENTS.md — crabjar-host-core (host-core)

> Purpose: Host runtime core — event bus, plugin API, WorkItem model, and config.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- Event bus for inter-crate communication
- Plugin API (trait definitions, lifecycle)
- WorkItem model (the unit of work for the agent loop)
- Config resolution

## Key Files

- `src/lib.rs` — crate entry point
- `src/event_bus.rs` — event bus implementation
- `src/plugin.rs` — plugin API
- `src/work_item.rs` — WorkItem model
- `src/config.rs` — config resolution

## Dependencies

- tokio, serde, serde_json, thiserror, uuid, chrono, toml, tracing, async-trait, futures, tempfile, path-absolutize

## Pitfalls

- host-core is the foundation for all host-* crates — keep it minimal
- Plugin API must be versioned to avoid breaking host plugins
- WorkItem model is the core data flow — changes here ripple across all host crates
