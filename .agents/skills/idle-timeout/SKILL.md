---
name: idle-timeout
description: |
  Implement idle timeout for daemon resource management. Use whenever the user wants to auto-shutdown daemon after inactivity — idle timeout prevents resource waste from persistent daemon sessions. Trigger when the user mentions idle timeout, daemon shutdown, resource management, auto-shutdown, or wants to build daemon resource management tooling.
---

# Idle Timeout

## Core Concept

Idle timeout auto-shutdowns daemon after inactivity. This prevents resource waste from persistent daemon sessions — the daemon survives between commands but shuts down when idle.

## Implementation

### 1. Idle Detection

Detect daemon idle state:
- **Last command**: timestamp of last command
- **Current time**: current timestamp
- **Elapsed**: time since last command
- **Threshold**: idle timeout threshold

### 2. Timeout Configuration

Configure idle timeout:
- **Threshold**: `AGENT_BROWSER_IDLE_TIMEOUT_MS` env var
- **Default**: configurable default value
- **Override**: CLI flag override
- **Session**: session-specific timeout

### 3. Shutdown Process

Shutdown daemon on idle:
- **Browser**: shutdown browser instance
- **Storage**: save session state if configured
- **Cleanup**: cleanup resources
- **Log**: log shutdown with reason

### 4. Restart Process

Restart daemon on command:
- **First command**: auto-start daemon
- **Session**: restore session state if configured
- **Browser**: launch browser instance
- **Connection**: establish CDP connection

## Configuration

- **Timeout value**: milliseconds, configurable
- **Env var**: `AGENT_BROWSER_IDLE_TIMEOUT_MS`
- **CLI flag**: override timeout
- **Session**: session-specific timeout
- **Auto-save**: save state on shutdown
- **Auto-restore**: restore state on restart

## Integration with ${PROJECT}

${PROJECT}'s resource management should adopt:
- Idle timeout for daemon auto-shutdown
- Threshold configuration via env var
- Auto-save/restore on shutdown/restart
- Resource cleanup on idle
- Cost awareness for persistent sessions
