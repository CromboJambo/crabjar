# AGENTS.md — crabjar-host-system (host-system)

> Purpose: System integration — system tray, notifications, clipboard, and secrets.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- System tray management
- Notification delivery
- Clipboard read/write
- OS keychain integration (via keyring crate)

## Key Files

- `src/lib.rs` — crate entry point
- `src/tray.rs` — system tray
- `src/notifications.rs` — notification delivery
- `src/clipboard.rs` — clipboard access
- `src/secrets.rs` — keychain integration

## Dependencies

- tokio, serde, serde_json, tracing, uuid, chrono, thiserror, libnotify, arboard, keyring, tempfile, crabjar-host-core

## Pitfalls

- Keychain access requires OS-specific handling (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Clipboard operations are synchronous — avoid blocking the event loop
- Notifications should be fire-and-forget — never block on delivery
