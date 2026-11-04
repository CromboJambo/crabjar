# Crabjar User Guide

This guide covers crabjar's core features and common workflows.

## Getting Started

```bash
# Check installation
crabjar --version

# Run system health check
crabjar doctor check

# List available commands
crabjar --help
```

## State Management

Crabjar uses state-docs to track the current state of your workspace and projects.

### Create a State Doc

```bash
# From a directory listing
crabjar state create my-project /path/to/project

# Manual creation
echo "# My Project State" > .crabjar_state.md
echo "Last updated: $(date)" >> .crabjar_state.md
```

### Manage State Docs

```bash
# List all state docs
crabjar state list

# Show a specific state doc
crabjar state show my-project

# Update a state doc
crabjar state update my-project "Added new feature"

# Check staleness
crabjar state staleness my-project
```

### State Doc Format

State docs are Markdown files with YAML frontmatter:

```markdown
---
name: my-project
created: 2026-11-03T12:00:00Z
updated: 2026-11-03T14:30:00Z
version: 1.2.3
status: active
---

# My Project State

## Current Status
Working on feature X...

## Recent Changes
- Added Y (2026-11-03)
- Fixed Z (2026-11-02)
```

## Execution Pipeline

Crabjar's execution pipeline provides gated, auditable command execution.

### Basic Execution

```bash
# Execute a simple command
crabjar exec "echo hello"

# Execute with timeout
crabjar exec --timeout 60 "long-running-command"

# Execute in sandbox
crabjar exec --sandbox "risky-command"
```

### Guard Approval Flow

Commands requiring approval are queued for review:

```bash
# List pending approvals
crabjar guard list-pending

# Approve a specific action
crabjar guard approve <action-id>

# Reject an action
crabjar guard reject <action-id> "reason"
```

### Trust Layers

Crabjar uses a 4-level trust system:

| Level | Name | Description |
|-------|------|-------------|
| 0 | Untrusted | New agents, requires approval for all actions |
| 1 | Trusted | Can execute low-risk actions without approval |
| 2 | Privileged | Can execute most actions, some require approval |
| 3 | Root | Full execution rights (use sparingly) |

```bash
# Check current trust layer
crabjar guard trust-layer

# Upgrade trust level (requires approval)
crabjar guard upgrade-trust <agent-id>
```

## Telemetry & Flight Recorder

Crabjar records all execution activity for debugging and auditing.

### View Recent Activity

```bash
# List recent events
crabjar telemetry list --limit 20

# Show details of specific event
crabjar telemetry show <event-id>

# Filter by type
crabjar telemetry list --type exec
```

### Telemetry Data

Each event includes:
- Timestamp
- Event type (exec, guard, trust, etc.)
- Agent ID
- Command/action details
- Outcome (success/failure)
- Duration
- Resource usage

## Knowledge Store

Crabjar provides a persistent knowledge store for agent memory.

### Store Information

```bash
# Add a fact
crabjar knowledge add "API endpoint" "https://api.example.com/v1"

# Add structured data
crabjar knowledge add-json "config" '{"host":"localhost","port":8080}'
```

### Query Knowledge

```bash
# Search by keyword
crabjar knowledge search "endpoint"

# Get specific fact
crabjar knowledge get "API endpoint"

# List all facts
crabjar knowledge list
```

## Tool Registry

Crabjar maintains a registry of available tools and their capabilities.

### Browse Tools

```bash
# List registered tools
crabjar tools list

# Show tool details
crabjar tools show <tool-name>

# Search by capability
crabjar tools search "web scraping"
```

## Sandboxing

Crabjar provides multiple isolation levels for agent execution.

### Unix User Isolation

```bash
# Create sandbox user
crabjar sandbox create-user crabjar-agent

# Execute as sandbox user
crabjar sandbox exec crabjar-agent "command"
```

### Container Isolation (Podman/Docker)

```bash
# Create container sandbox
crabjar sandbox create-container my-sandbox ubuntu:22.04

# Execute in container
crabjar sandbox exec-container my-sandbox "command"

# Remove container
crabjar sandbox remove-container my-sandbox
```

### VM Isolation

For maximum isolation, use VM-based sandboxes (requires vm-bridge).

## Fleet Coordination

Crabjar can coordinate across multiple machines in a fleet.

### Register as Worker

```bash
# Register with conductor
crabjar fleet register --conductor http://conductor:8091

# Report capabilities
crabjar fleet capabilities add "web-scraping" "llm-inference"
```

### Task Assignment

Workers receive tasks from the conductor based on their capabilities and trust level.

## Skills

Crabjar supports executable skills that extend its capabilities.

### List Available Skills

```bash
crabjar skills list
```

### Execute a Skill

```bash
crabjar skills run <skill-name> --arg value
```

### Create Custom Skills

Skills are shell scripts or Rust binaries in the `skills/` directory:

```bash
# Create skill directory
mkdir -p skills/my-skill

# Add executable script
cat > skills/my-skill/run.sh <<'EOF'
#!/bin/bash
echo "Hello from my skill!"
EOF
chmod +x skills/my-skill/run.sh
```

## Doctor & Diagnostics

The doctor command provides system health checks and diagnostics.

### Run Full Check

```bash
crabjar doctor check
```

This checks:
- Binary installation
- Configuration file
- Database connectivity
- Tool runtime availability (podman, docker, etc.)
- Network connectivity
- Disk space
- SSH agent status

### Individual Checks

```bash
# Check specific component
crabjar doctor check --component config
crabjar doctor check --component database
crabjar doctor check --component network
```

## Configuration

Crabjar's behavior is controlled by `.crabjar_config.toml`.

### Default Config Location

```bash
# Current directory
./.crabjar_config.toml

# Home directory fallback
~/.crabjar_config.toml
```

### Example Configuration

```toml
[identity]
machine_id = "my-machine"
version = "0.12.5"

[guard]
trust_layer = 0
auto_grant = false

[execution]
container_runtime = "podman"
sandbox_user = "crabjar-agent"
timeout_seconds = 300

[telemetry]
enabled = true
log_level = "info"
output_dir = "./data/logs"
```

## Tips & Best Practices

1. **Use state-docs regularly**: Update them as your project evolves to maintain accurate provenance.

2. **Start with trust layer 0**: New agents should begin untrusted and earn higher trust levels through consistent behavior.

3. **Review pending approvals promptly**: The guard approval flow is only effective if someone reviews the queue.

4. **Enable telemetry in production**: The flight recorder provides invaluable debugging information.

5. **Use sandboxes for untrusted code**: Always sandbox execution of code from unknown sources.

6. **Backup your knowledge store**: Regularly export important facts and configurations.

7. **Monitor fleet health**: Use `crabjar doctor check` regularly on all fleet nodes.

## Troubleshooting

### Command Not Found

```bash
# Check installation
which crabjar

# If not installed, build from source
cargo install --path . -p crabjar
```

### Configuration Errors

```bash
# Validate config file
crabjar doctor check --component config

# Reset to defaults
rm .crabjar_config.toml
```

### Database Issues

```bash
# Check database connectivity
crabjar doctor check --component database

# Rebuild state docs index
crabjar state rebuild-index
```

### Network Problems

```bash
# Check network connectivity
crabjar doctor check --component network

# Test SSH agent
ssh-add -l
```

## Further Reading

- [Architecture Decision Records](../specs/) — Technical design decisions
- [Project Map](../project_map.md) — Complete codebase structure
- [Roadmap](../ROADMAP.md) — Development priorities and status
- [Deployment Guide](./deployment.md) — Installing crabjar on different environments
