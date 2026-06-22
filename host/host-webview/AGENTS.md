# AGENTS.md — crabjar-host-webview (host-webview)

> Purpose: WebView session management, OAuth2, and token cache.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- WebView session lifecycle (create, manage, destroy)
- OAuth2 flow (authorization code, token exchange)
- Token cache (SQLite-backed, keyring-suppressed for tests)

## Key Files

- `src/lib.rs` — crate entry point
- `src/controller.rs` — WebView session controller
- `src/partition.rs` — WebView partition management
- `src/cookie_store.rs` — cookie store
- `src/auth.rs` — OAuth2 flow
- `src/token_cache.rs` — token persistence

## Dependencies

- tokio, serde, serde_json, tracing, uuid, chrono, thiserror, crabjar-host-core, rusqlite, keyring, sha2, base64, tempfile

## Pitfalls

- Multiple unused imports and variables in auth.rs and token_cache.rs — clean up
- `mut tx` in cookie_store.rs is never mutated — remove mut qualifier
- `cookie_store` field in AuthManager is never read — verify if it's needed
- `save_tokens` method in AuthManager is never used — verify if it's dead code
- Token cache uses keyring for OS-level suppression in tests
- OAuth2 state parameter must be cryptographically random
