# Crabjar API Documentation

This document covers crabjar's programmatic APIs for building integrations and extensions.

## Overview

Crabjar exposes several APIs:

1. **CLI Interface** — Command-line tool for direct interaction
2. **Conductor HTTP API** — Fleet orchestration REST endpoints
3. **Worker Protocol** — Worker registration and task assignment protocol
4. **MCP Server Interfaces** — Model Context Protocol servers for AI integration
5. **Library APIs** — Rust crates for embedding crabjar functionality

## CLI Interface

The primary interface is the `crabjar` binary:

```bash
crabjar <command> [subcommand] [options]
```

### Command Structure

All commands return structured JSON on stdout:

```json
{
  "success": true,
  "data": { ... },
  "doubt": {
    "assumptions": [],
    "blind_spots": [],
    "last_validation": "2026-11-03T12:00:00Z",
    "stale_after": "2026-11-10T12:00:00Z"
  }
}
```

Error responses:

```json
{
  "success": false,
  "error": "Error message",
  "usage": ["crabjar command [options]"]
}
```

### Key Commands

| Command | Description |
|---------|-------------|
| `state` | State-docs management |
| `exec` | Execution pipeline |
| `guard` | Trust layer and approval management |
| `telemetry` | Flight recorder access |
| `knowledge` | Knowledge store operations |
| `tools` | Tool registry management |
| `sandbox` | Agent isolation management |
| `fleet` | Fleet coordination |
| `skills` | Skill execution and management |
| `doctor` | System diagnostics |

## Conductor HTTP API

The crabjar-conductor service provides a REST API for fleet orchestration. Default port: 8091.

### Worker Registration

```http
POST /api/v1/workers/register
Content-Type: application/json

{
  "worker_id": "unique-worker-identifier",
  "capabilities": ["web-scraping", "llm-inference"],
  "trust_level": 0,
  "metadata": {
    "hostname": "worker-node-1",
    "os": "linux",
    "arch": "x86_64"
  }
}
```

Response:

```json
{
  "success": true,
  "data": {
    "worker_id": "unique-worker-identifier",
    "status": "registered",
    "token": "jwt-token-for-authentication"
  }
}
```

### Task Assignment

```http
POST /api/v1/tasks/assign
Content-Type: application/json

{
  "task": {
    "type": "web-scraping",
    "params": {
      "url": "https://example.com",
      "selector": ".content"
    }
  },
  "target_capabilities": ["web-scraping"],
  "priority": 1
}
```

Response:

```json
{
  "success": true,
  "data": {
    "task_id": "task-uuid",
    "assigned_worker": "worker-id",
    "status": "assigned"
  }
}
```

### Task Status

```http
GET /api/v1/tasks/{task_id}/status
Authorization: Bearer <token>
```

Response:

```json
{
  "success": true,
  "data": {
    "task_id": "task-uuid",
    "status": "completed",
    "result": { ... },
    "duration_ms": 1500,
    "worker_id": "worker-id"
  }
}
```

### Worker Heartbeat

```http
POST /api/v1/workers/{worker_id}/heartbeat
Authorization: Bearer <token>
Content-Type: application/json

{
  "status": "idle",
  "current_task": null,
  "last_activity": "2026-11-03T12:00:00Z"
}
```

### Fleet Status

```http
GET /api/v1/fleet/status
Authorization: Bearer <token>
```

Response:

```json
{
  "success": true,
  "data": {
    "total_workers": 4,
    "idle_workers": 3,
    "busy_workers": 1,
    "workers": [
      {
        "worker_id": "worker-1",
        "status": "idle",
        "capabilities": ["web-scraping"],
        "last_heartbeat": "2026-11-03T12:00:00Z"
      }
    ]
  }
}
```

## Worker Protocol

Workers communicate with the conductor using a simple JSON-RPC-style protocol over HTTP.

### Registration Flow

1. Worker sends POST to `/api/v1/workers/register`
2. Conductor validates capabilities and returns JWT token
3. Worker stores token for subsequent requests
4. Worker begins sending heartbeats every 30 seconds

### Task Execution Flow

1. Conductor assigns task via `/api/v1/tasks/assign`
2. Worker acknowledges receipt
3. Worker executes task in sandboxed environment
4. Worker reports result via POST to `/api/v1/tasks/{task_id}/result`
5. Conductor updates task status and notifies requester

### Error Handling

Workers report errors as part of the task result:

```json
{
  "status": "failed",
  "error": {
    "type": "timeout",
    "message": "Task exceeded 300s timeout",
    "details": { ... }
  }
}
```

## MCP Server Interfaces

Crabjar provides several MCP (Model Context Protocol) servers for AI integration.

### crabjar-mcp-tool-registry

Provides tool discovery and invocation capabilities to LLM-powered agents.

**Endpoints:**
- `list_tools` — Enumerate available tools
- `get_tool_schema` — Get JSON schema for a tool
- `invoke_tool` — Execute a tool with parameters

### crabjar-compress-mcp

Context compression services for optimizing LLM context windows.

**Endpoints:**
- `compress_context` — Compress conversation history
- `summarize` — Generate summary of content
- `extract_key_points` — Identify important information

### crabjar-unsloth-mcp

Integration with Unsloth Studio for model management and training.

**Endpoints:**
- `list_models` — List available models
- `load_model` — Load a model into inference server
- `unload_model` — Release model resources
- `start_training` — Begin fine-tuning job

### crabjar-herdr-mcp

Integration with Herdr terminal multiplexer for remote session management.

**Endpoints:**
- `list_sessions` — List active sessions
- `create_session` — Start new terminal session
- `send_command` — Execute command in session
- `read_output` — Get session output

## Library APIs (Rust)

Crabjar's functionality is exposed through individual Rust crates.

### crabjar-guard

Execution gate and trust layer management.

```rust
use crabjar_guard::{Guard, TrustLayer, Action};

let guard = Guard::new("/path/to/guard.db");

// Check if action is allowed
let action = Action {
    command: "rm -rf /tmp/test",
    risk_level: RiskLevel::Low,
};

match guard.evaluate(&action) {
    GateResult::Proceed => { /* execute */ }
    GateResult::PendingApproval(id) => { /* queue for approval */ }
    GateResult::Denied(reason) => { /* log and report */ }
}
```

### crabjar-telemetry

Flight recorder for execution activity.

```rust
use crabjar_telemetry::{Telemetry, Event, EventType};

let telemetry = Telemetry::new("/path/to/flight.db");

telemetry.record(Event {
    event_type: EventType::Exec,
    agent_id: "agent-1",
    command: "ls -la",
    outcome: Outcome::Success,
    duration_ms: 50,
});

// Query recent events
let events = telemetry.recent(20);
```

### crabjar-sandbox

Agent isolation management.

```rust
use crabjar_sandbox::{SandboxManager, SandboxConfig};

let manager = SandboxManager::new();

// Create Unix user sandbox
let config = SandboxConfig {
    isolation_type: IsolationType::UnixUser,
    username: "crabjar-agent",
};

let sandbox = manager.create("agent-1", config)?;
sandbox.execute("echo hello")?;
```

### crabjar-memory

Persistent knowledge store.

```rust
use crabjar_memory::{MemoryStore, FactType};

let store = MemoryStore::new("/path/to/memory.db");

// Store a fact
store.add_fact("api_endpoint", "https://api.example.com/v1", FactType::URL);

// Query facts
let endpoint = store.get_fact("api_endpoint")?;
```

## Authentication & Security

### JWT Tokens

Conductor API uses JWT tokens for authentication. Tokens are issued during worker registration and expire after 24 hours.

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

### SSH Keys

Worker-to-conductor communication can optionally use SSH keys for additional security.

### TLS/SSL

Production deployments should use TLS for all HTTP endpoints. The conductor supports both self-signed and CA-signed certificates.

## Rate Limiting

API endpoints are rate-limited to prevent abuse:

- Worker registration: 10 requests/minute
- Task assignment: 100 requests/minute
- Heartbeat: 60 requests/minute per worker
- Fleet status: 30 requests/minute

Rate limit headers are included in responses:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1700000000
```

## Error Codes

| Code | Description |
|------|-------------|
| 400 | Bad request (malformed JSON, missing fields) |
| 401 | Unauthorized (invalid or expired token) |
| 403 | Forbidden (insufficient permissions) |
| 404 | Not found (resource doesn't exist) |
| 409 | Conflict (duplicate registration) |
| 422 | Unprocessable entity (validation error) |
| 429 | Too many requests (rate limit exceeded) |
| 500 | Internal server error |
| 503 | Service unavailable (conductor restarting) |

## Examples

### Register and Execute Task (Python)

```python
import requests

# Register worker
reg = requests.post("http://conductor:8091/api/v1/workers/register", json={
    "worker_id": "my-worker",
    "capabilities": ["web-scraping"],
    "trust_level": 0
})
token = reg.json()["data"]["token"]

headers = {"Authorization": f"Bearer {token}"}

# Assign task
task = requests.post("http://conductor:8091/api/v1/tasks/assign", headers=headers, json={
    "task": {
        "type": "web-scraping",
        "params": {"url": "https://example.com"}
    },
    "target_capabilities": ["web-scraping"]
})
task_id = task.json()["data"]["task_id"]

# Check status
status = requests.get(f"http://conductor:8091/api/v1/tasks/{task_id}/status", headers=headers)
print(status.json())
```

### CLI Automation (Bash)

```bash
#!/bin/bash
# Automated fleet health check

for host in lappy artix-box ftw3; do
    echo "Checking $host..."
    ssh -i ~/.ssh/id_lab crombo@$host /opt/crabjar/crabjar doctor check
done
```

## Further Reading

- [Conductor Source Code](../crates/crabjar-conductor/) — Implementation details
- [Worker Source Code](../crates/crabjar-worker/) — Worker protocol implementation
- [Guard Source Code](../guard/) — Execution gate implementation
- [MCP Server Documentation](https://modelcontextprotocol.io/) — MCP specification
