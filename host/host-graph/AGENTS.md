# AGENTS.md — host-graph (host-graph)

> Purpose: Microsoft Graph API client for calendar, email, and contact integration.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- Microsoft Graph API client
- Calendar operations (create, read, update, delete events)
- Email operations (send, read, list messages)
- Contact operations

## Key Files

- `src/lib.rs` — crate entry point
- `src/client.rs` — Graph API client
- `src/calendar.rs` — calendar operations
- `src/email.rs` — email operations
- `src/contacts.rs` — contact operations

## Dependencies

- anyhow, reqwest, serde, serde_json, tokio, tracing, thiserror, chrono, async-trait, futures, uuid, toml

## Pitfalls

- Graph API rate limits — implement exponential backoff
- OAuth2 token refresh must be handled transparently
- Pagination through Graph API results (odata.nextLink)
