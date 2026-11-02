# Home Assistant Application-Layer Tools (P2-2)

**Date:** November 2, 2026
**Status:** Draft
**Priority:** P2 — Productivity improvement (not a drop-in replacement blocker)

## Problem Statement

Crabjar has MQTT infrastructure (`host-mqtt`) but lacks Home Assistant-specific tooling. Agents cannot interact with Home Assistant instances directly to:
- Query device/entity states
- Trigger services (turn lights on, lock doors, etc.)
- Discover available entities and services

Hermes provides `homeassistant` tools via MCP integration. Crabjar needs equivalent built-in tools for home-lab management workflows.

## Solution Approach

Create a `crabjar-homeassistant` crate that provides:
1. **HA REST API client** — communicate with Home Assistant instances via HTTP REST API
2. **Tool implementations** — `ha_get_state`, `ha_call_service`, `ha_list_entities`, `ha_list_services`
3. **Orchestrator integration** — register tools in the orchestrator's tool dispatch loop

## API Design

### ha_get_state(entity_id)
- **Input:** Entity ID (e.g., "light.living_room")
- **Output:** JSON with entity state, attributes, last_updated, last_changed
- **Example:** `{"entity_id": "light.living_room", "state": "on", "attributes": {"brightness": 255}, ...}`

### ha_call_service(service_domain, service_name, service_data)
- **Input:** Service domain (e.g., "light"), service name (e.g., "turn_on"), optional data
- **Output:** JSON with success status
- **Example:** `ha_call_service("light", "turn_on", {"entity_id": "light.living_room"})`

### ha_list_entities(domain)
- **Input:** Optional domain filter (e.g., "light", "switch")
- **Output:** JSON array of entity IDs in that domain
- **Example:** `["light.living_room", "light.kitchen"]`

### ha_list_services()
- **Input:** None
- **Output:** JSON object mapping service domains to available services
- **Example:** `{"light": ["turn_on", "turn_off"], "switch": ["turn_on", "turn_off"]}`

## Configuration

Home Assistant connection details via environment variables:
- `CRABJAR_HA_URL` — Home Assistant base URL (default: http://localhost:8123)
- `CRABJAR_HA_TOKEN` — Long-lived access token for authentication

## Dependencies

- `reqwest` — HTTP client for REST API calls
- `serde_json` — JSON serialization/deserialization

## Testing Strategy

1. Unit tests with mock HTTP responses
2. Integration tests against a running Home Assistant instance (optional)
3. Error handling: timeout, authentication failure, invalid entity/service IDs

## Success Criteria

- [ ] `crabjar-homeassistant` crate created with HA REST API client
- [ ] Four tools implemented: ha_get_state, ha_call_service, ha_list_entities, ha_list_services
- [ ] Tools integrated into orchestrator's tool dispatch loop
- [ ] Unit tests passing for all four tools
- [ ] Configuration via environment variables documented

## Drop-in Replacement Context

This matches Hermes' `homeassistant` MCP tools. Once implemented, crabjar agents can manage Home Assistant instances with the same capability as Hermes agents using the homeassistant MCP server.
