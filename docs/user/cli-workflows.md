# Crabjar CLI Workflows Guide

This guide covers common crabjar command-line workflows for day-to-day agent operations.

## Quick Start

```bash
# Check system state
crabjar state list

# Show detailed info about a specific state doc
crabjar state show <doc_name>

# Execute an action through the pipeline
crabjar exec shell "echo hello world"

# List available MCP tools
crabjar tool list

# Search for tools by capability
crabjar tool search "web scraping"
```

## State Documentation Management

State docs are durable Markdown files that track system state, decisions, and knowledge.

### List All State Docs

```bash
crabjar state list
```

Returns a JSON array of all state doc names with metadata.

### Show State Doc Details

```bash
crabjar state show <doc_name>
```

Displays the full content and metadata of a specific state doc.

### Annotate State Doc

Add annotations to track changes or decisions:

```bash
crabjar state annotate <doc_name> "Updated deployment config for v2.1"
```

### Check Staleness

Determine if a state doc needs updating:

```bash
crabjar state staleness <doc_name>
```

Returns structured JSON with status (fresh/stale/expired/moldy), age in days, and trustworthiness indicators.

## Knowledge Store Operations

The knowledge store manages facts, patterns, and rules as a graph of interconnected nodes.

### Sync Knowledge Store

```bash
crabjar knowledge sync
```

Synchronizes the local knowledge store with any remote sources or updates.

### Query Knowledge Store

Search for relevant knowledge:

```bash
# Search by topic
crabjar knowledge query "deployment procedures"

# Search with filters
crabjar knowledge query "API endpoints" --category api --limit 10

# Verify knowledge freshness
crabjar knowledge verify <fact_id>
```

## Execution Gate Approvals

When an action requires approval (based on trust layer or risk assessment), it enters the pending queue.

### View Pending Queue

```bash
crabjar guard queue
```

Lists all actions waiting for approval with their risk assessments and proposed commands.

### Approve Action

```bash
crabjar guard approve <action_id>
```

Approves a specific pending action for execution.

### Reject Action

```bash
crabjar guard reject <action_id> "Reason for rejection"
```

Rejects a pending action with an optional reason.

## Executing Actions

The `exec` command runs actions through the complete crabjar pipeline (guard → concierge → telemetry → outcome).

### Shell Commands

```bash
# Simple command
crabjar exec shell "echo hello"

# Command with arguments
crabjar exec shell "ls -la /tmp"

# Complex commands with pipes
crabjar exec shell "ps aux | grep crabjar | wc -l"
```

### File Operations

```bash
# Write file content
crabjar exec write_file "/tmp/test.txt" "Hello World"

# Read file (via shell)
crabjar exec shell "cat /tmp/test.txt"
```

### Browser Automation

```bash
# Navigate to URL and capture screenshot
crabjar exec browser_navigate "https://example.com" --screenshot /tmp/screenshot.png

# Click element
crabjar exec browser_click "#submit-button"

# Fill form field
crabjar exec browser_fill "#email-input" "user@example.com"
```

### Web Operations

```bash
# Search the web
crabjar exec web_search "latest crabjar documentation" --limit 5

# Extract content from URL
crabjar exec web_extract "https://example.com/article" --char-limit 10000
```

## MCP Tool Management

### List All Tools

```bash
crabjar tool list
```

Shows all available tools with their categories and descriptions.

### Search for Tools

```bash
# Search by keyword
crabjar tool search "database"

# Search with filters
crabjar tool search "API" --category integration --limit 20
```

### Install Tool

```bash
crabjar tool install <tool_name_or_url>
```

Installs a new MCP tool from the registry or a direct URL.

### Tool Categories

- **shell**: Shell command execution and file operations
- **browser**: Web browser automation (navigation, clicking, form filling)
- **web**: Web search and content extraction
- **integration**: Third-party service integrations (GitHub, Discord, etc.)
- **analysis**: Code analysis, security scanning, performance profiling

## Trust Layer Management

Trust layers (0-3) determine which actions require approval. Higher trust = more autonomy.

### View Current Trust Layer

```bash
crabjar state show trust-layer
```

### Understanding Trust Layers

| Level | Name | Autonomy | Approval Required For |
|-------|------|----------|----------------------|
| 0 | Observer | Read-only | All actions |
| 1 | Executor | Limited write | Destructive operations |
| 2 | Operator | Full CRUD | Cross-environment changes |
| 3 | Administrator | Complete control | None (rarely) |

## Example Workflows

### Deploy a Service

```bash
# Check current deployment state
crabjar state show deployment-config

# Execute deployment commands
crabjar exec shell "cd /opt/myapp && git pull origin main"
crabjar exec shell "cd /opt/myapp && cargo build --release"
crabjar exec shell "systemctl restart myapp"

# Verify deployment
crabjar exec shell "systemctl status myapp"

# Update state documentation
crabjar state annotate deployment-config "Deployed v2.1.0 at $(date)"
```

### Debug a Production Issue

```bash
# Check system health
crabjar exec shell "df -h /var/log/myapp"
crabjar exec shell "tail -n 50 /var/log/myapp/error.log"

# Search knowledge base for similar issues
crabjar knowledge query "high memory usage production"

# Execute diagnostic commands
crabjar exec shell "ps aux | grep myapp | head -5"
```

### Manage MCP Tools Across Fleet

```bash
# Check which tools are installed
crabjar tool list --category integration

# Install new tool across all workers
for worker in $(crabjar fleet list-workers); do
    crabjar --target $worker tool install github-integration
done

# Verify installation
crabjar tool search "github"
```

## Output Format

All crabjar commands output structured JSON to stdout:

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "metadata": {
    "timestamp": "2026-11-04T12:00:00Z",
    "duration_ms": 150
  }
}
```

Error responses include:
```json
{
  "success": false,
  "error": "Description of the error",
  "usage": ["command", "arg1", "arg2"]
}
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CRABJAR_CONFIG` | Path to configuration file | `~/.crabjar/config.toml` |
| `CRABJAR_LOG_LEVEL` | Log verbosity (debug/info/warn/error) | `info` |
| `CRABJAR_TIMEOUT` | Command timeout in seconds | `300` |