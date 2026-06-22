# AGENTS.md — host-mqtt (host-mqtt)

> Purpose: MQTT client + Home Assistant discovery for home automation integration.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- MQTT connection management
- Home Assistant device discovery (auto-discovery payloads)
- Message publish/subscribe

## Key Files

- `src/lib.rs` — crate entry point
- `src/client.rs` — MQTT client
- `src/discovery.rs` — Home Assistant discovery
- `src/config.rs` — MQTT configuration

## Dependencies

- rumqttc, toml, serde, serde_json, tokio, tracing, thiserror, chrono, uuid, async-trait, futures, tempfile

## Pitfalls

- MQTT connection should handle reconnection gracefully
- Home Assistant discovery payloads must follow HA's schema
- Message QoS levels matter for reliability vs. performance tradeoffs
