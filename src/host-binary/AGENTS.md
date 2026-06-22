# AGENTS.md — crabjar-host (host-binary)

> Purpose: Host binary — TUI application that wires together all host runtime crates.

## Layer

Layer 5: product — product-facing crates, may depend on layers 0, 1, 2, 3, 4, 5.

## Public API

- TUI application entry point
- Host runtime initialization (wires host-core, host-system, host-observe, host-agent, host-webview)
- Teams plugin integration

## Key Files

- `src/lib.rs` — crate entry point
- `src/main.rs` — TUI entry point
- `src/app.rs` — application composition

## Dependencies

- tokio, clap, serde, serde_json, tracing, tracing-subscriber, chrono, uuid, toml, crabjar-host-core, crabjar-host-system, crabjar-host-observe, crabjar-host-agent, crabjar-host-webview, crabjar-app-teams, ratatui, crossterm, thiserror, async-trait, dirs, tempfile

## Pitfalls

- This is the composition root for the host runtime — wire crates here
- TUI uses ratatui + crossterm — keep UI logic separate from business logic
- Host binary depends on all host-* crates — changes here ripple across the host layer
