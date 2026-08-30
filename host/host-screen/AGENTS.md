# AGENTS.md — host-screen (host-screen)

> Purpose: Screen capture and display protocol integration for VM bridge.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- Screen capture (PipeWire for Wayland, X11 screen grab for X11)
- Display protocol integration (via vm-bridge)
- Audio capture (microphone + system audio)

## Key Files

- `src/lib.rs` — crate entry point
- `src/capture.rs` — screen capture logic
- `src/protocol.rs` — display protocol handling
- `src/audio.rs` — audio capture

## Dependencies

- tokio, async-trait, thiserror, tracing, anyhow, serde, serde_json, reqwest, futures-core, rtp

## Pitfalls

- vm-bridge (axum-mux) is a binary-only native crate (no lib target) — do not add it as a library dependency; it is not used by this crate
- Screen capture requires different implementations for X11 vs. Wayland
- PipeWire is the primary capture source on Wayland
- Audio capture needs separate handling for microphone vs. system audio
