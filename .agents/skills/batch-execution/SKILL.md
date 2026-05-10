---
name: batch-execution
description: |
  Implement batch execution for multi-step workflow efficiency. Use whenever the user wants to execute multiple web actions without per-command process startup overhead — batch command avoids repeated process creation for multi-step workflows. Trigger when the user mentions batch execution, multi-step workflows, workflow efficiency, batch commands, or wants to build multi-step web action tooling.
---

# Batch Execution

## Core Concept

Batch execution avoids per-command process startup overhead for multi-step workflows. Instead of creating a new process for each command, batch executes multiple actions in a single daemon session.

## Batch Format

### Argument Mode

```
batch click @e1 fill @e2 "value" get text @e3
```

### stdin JSON Mode

```json
[
  {"action": "click", "ref": "@e1"},
  {"action": "fill", "ref": "@e2", "value": "value"},
  {"action": "get text", "ref": "@e3"}
]
```

## Implementation

### 1. Batch Parsing

Parse batch commands:
- **Argument mode**: sequential command parsing
- **JSON mode**: structured command array
- **Mixed mode**: argument + JSON input

### 2. Execution Pipeline

Execute batch in single session:
- **Daemon**: existing daemon session (no startup)
- **IPC**: single IPC connection
- **Commands**: sequential execution
- **Results**: aggregated output

### 3. Bail Option

`--bail` option for error handling:
- **Bail on error**: stop execution on first failure
- **Continue**: continue execution despite failures
- **Partial**: return partial results

### 4. Output Aggregation

Aggregate batch results:
- **JSON output**: structured result array
- **Ref alignment**: results mapped to refs
- **Error logging**: failures logged with reason
- **Success count**: number of successful actions

## Configuration

- **Bail mode**: bail on error, continue, partial
- **Output format**: JSON, structured, aggregated
- **Ref alignment**: mandatory, optional
- **Timeout**: per-command timeout, batch timeout
- **Thread count**: serial, parallel execution

## Integration with Crabjar

Crabjar's multi-step workflow should adopt:
- Batch execution for workflow efficiency
- Daemon persistence (no per-command startup)
- JSON output for structured results
- Bail option for error handling
- Ref alignment for result mapping
- Aggregated output for workflow summary

## References

Read `state-docs/agent-browser-state.md` section 2.4 for CLI command surface and section 7.5 for stale detection thresholds.
