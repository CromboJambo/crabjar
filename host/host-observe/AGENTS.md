# AGENTS.md — crabjar-host-observe (host-observe)

> Purpose: Metrics, tracing, and health reporting for the host runtime.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- Metrics collection and export
- Tracing setup (EnvFilter, layered subscribers)
- Health reporting (liveness, readiness)

## Key Files

- `src/lib.rs` — crate entry point
- `src/metrics.rs` — metrics collection
- `src/tracing_setup.rs` — tracing configuration
- `src/health.rs` — health reporting

## Dependencies

- tokio, serde, serde_json, tracing, tracing-subscriber, tracing-appender, tracing-error, chrono, uuid, thiserror, tempfile, crabjar-host-core

## Pitfalls

- `info!`/`warn!` corrupts REPL/TUI — use `debug!` for internal diagnostics
- Tracing env filter uses `TRACING_LEVEL` env var
- Metrics export should not block the event loop
- Variable `env_filter` in tracing_setup.rs is assigned but unused — verify intent
