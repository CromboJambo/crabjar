# AGENTS.md — crabjar-telemetry (telemetry)

> Purpose: Flight recorder — command execution tracking, git state capture, and tool receipts.

## Layer

Layer 1: substrate — low-level storage, may depend on layer 0 only.

## Public API

- `FlightRecorder` — execute command + capture git dirty state + git diff
- `CommandExecutor` — process spawning + output capture
- Tool receipt generation (HMAC-SHA256 per invocation)

## Key Files

- `src/lib.rs` — crate entry point
- `src/flight_recorder.rs` — FlightRecorder (execute_command, git capture)
- `src/command_executor.rs` — process spawning + output capture
- `src/schema.rs` — SQLite schema
- `src/error.rs` — error types

## Dependencies

- tokio, serde, serde_json, bitcode, chrono, uuid, rusqlite, tracing, thiserror, cargo-declared, path-absolutize, sha2, hex, ignore, hmac, getrandom, base64, tempfile

## Pitfalls

- Tool receipts use HMAC-SHA256 — each invocation gets a unique, verifiable receipt
- Git dirty state and diff are captured at execution time, not after
- `crabjar doctor check` validates flight.db schema
- Receipt prefix constant `RECEIPT_PREFIX` is defined but may be unused — verify
