# bitwarden-mcp

MCP (Model Context Protocol) server for Bitwarden Secrets Manager, built with the official [Bitwarden Rust SDK](https://github.com/bitwarden/sdk-sm).

This replaces shelling out to the `bws` CLI with direct SDK integration — no subprocess overhead, structured error handling, and native async.

## Features

- **7 MCP tools**: `status`, `list_projects`, `list_secrets`, `get_secret`, `create_secret`, `update_secret`, `delete_secret`
- **Direct SDK auth**: Uses service account access tokens (no interactive login)
- **Config discovery**: Reads server URLs from `~/.config/bws/config` automatically
- **Env overrides**: All settings can be overridden via environment variables

## Setup

### 1. Get a Service Account Token

In your Bitwarden organization:
1. Go to **Organization Settings** → **Service Accounts**
2. Create a new service account (or use an existing one)
3. Copy the access token

### 2. Configure Environment

```bash
# Required: service account access token
export BWS_ACCESS_TOKEN="your-service-account-token"

# Optional overrides (defaults read from ~/.config/bws/config)
export BW_API_URL="https://vault.bitwarden.com/api"
export BW_IDENTITY_URL="https://vault.bitwarden.com/identity"
export BW_ORG_ID="your-organization-uuid"  # default org for all operations
```

### 3. Build

```bash
cd mcps/bitwarden-mcp
cargo build --release
```

## Usage as an MCP Server

Add to your MCP client config (e.g., Hermes Agent, Zed, etc.):

```json
{
  "mcpServers": {
    "bitwarden": {
      "command": "/path/to/bitwarden-mcp/target/release/bitwarden-mcp",
      "env": {
        "BWS_ACCESS_TOKEN": "${BITWARDEN_SERVICE_ACCOUNT_TOKEN}",
        "BW_ORG_ID": "your-org-uuid"
      }
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `status` | Check connectivity and auth status |
| `list_projects` | List all projects in the organization |
| `list_secrets` | List secret metadata (no values) |
| `get_secret` | Get a secret by ID or key (returns value) |
| `create_secret` | Create a new secret in a project |
| `update_secret` | Update an existing secret |
| `delete_secret` | Delete one or more secrets |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `BWS_ACCESS_TOKEN` or `BW_ACCESS_TOKEN` | Yes | Service account access token |
| `BW_API_URL` | No | API base URL (default: from bws config or bitwarden.com) |
| `BW_IDENTITY_URL` | No | Identity server URL (default: from bws config or bitwarden.com) |
| `BW_ORG_ID` | No | Default organization ID (can also be passed per-tool-call) |

## Dependencies

- [`bitwarden`](https://crates.io/crates/bitwarden) v2.1 — official Bitwarden Rust SDK
- [`rmcp`](https://crates.io/crates/rmcp) v3.5 — MCP protocol implementation
- [`tokio`](https://crates.io/crates/tokio) — async runtime
