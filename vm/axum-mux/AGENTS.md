# AGENTS.md — vm-bridge (axum-mux)

> Purpose: Per-VM websocket relay for display protocols — hardened byte transport.

## Layer

Layer 3: runtime — execution runtime, may depend on layers 0, 1, 2, 3.

## Public API

- WebSocket proxy for display protocol bytes
- Per-VM process isolation
- Hardened transport (no protocol parsing, just byte forwarding)

## Key Files

- `src/main.rs` — binary entry (supervisor/worker re-exec model)
- WebSocket proxy logic
- Process supervisor logic

## Dependencies

- tokio, axum, futures-util, serde, toml, anyhow, tracing, tracing-subscriber

## Pitfalls

- vm-bridge is binary-only (no lib target) but a *native* binary (tokio + axum, no wasm) — it builds fine on native. Do not add it as a library dependency; nothing embeds it (see ADR-005)
- Hardened by design: no protocol parsing, just byte transport
- Can be extended with screen sharing in the future
