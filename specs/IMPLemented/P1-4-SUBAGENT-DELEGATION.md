# P1-4: Subagent Delegation Framework

**Date:** October 31, 2026
**Status:** Implemented (November 2, 2026)
**Priority:** P1 (Drop-in Replacement Blocker)
**North Star:** Drop-in replacement for Hermes Agent — users migrating should find identical delegation behavior.

## Problem Statement

Hermes provides `delegate_task(tasks)` which spawns isolated subagents in parallel, each with its own conversation context, terminal session, and tool access. Crabjar has a conductor service for fleet coordination but lacks an agent-level task delegation paradigm. Users migrating from Hermes lose the ability to spawn parallel subagents for concurrent work.

## Scope

Build a `delegate_task` tool that:
1. Spawns isolated subagent tasks with separate context and terminal sessions
2. Supports single task delegation: `delegate_task(goal, context)`
3. Supports batch delegation: `delegate_task(tasks=[{goal, context}, ...])` running in parallel
4. Returns aggregated results when all children complete
5. Supports background execution with result delivery on completion
6. Integrates with crabjar's guard system for task authorization
7. Enforces concurrency limits and timeout management

## Out of Scope

- Durable work that outlives the process (use cronjob or terminal(background=True))
- Cross-machine delegation (that's the conductor's fleet coordination role)
- Recursive delegation beyond configurable depth limit

## Architecture

### Tool Schema (Hermes-Compatible)

```json
{
  "name": "delegate_task",
  "parameters": {
    "type": "object",
    "properties": {
      "goal": {
        "type": "string",
        "description": "What this subagent should accomplish"
      },
      "context": {
        "type": "string",
        "description": "Background information needed by the subagent"
      },
      "tasks": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "goal": {"type": "string"},
            "context": {"type": "string"}
          }
        },
        "description": "Batch of parallel tasks (overrides single goal/context)"
      },
      "background": {
        "type": "boolean",
        "default": false,
        "description": "Return immediately, deliver results when complete"
      },
      "model": {
        "type": "string",
        "description": "Override model for subagent(s)"
      }
    }
  }
}
```

### Response Format

```json
{
  "success": true,
  "results": [
    {
      "goal": "Task description",
      "summary": "Subagent's final result",
      "status": "completed"
    }
  ],
  "task_ids": ["task-uuid-1", "task-uuid-2"]
}
```

## Implementation Design

### New Crate: `crabjar-delegation`

```
crabjar-delegation/
├── src/
│   ├── lib.rs           # Public API
│   ├── dispatcher.rs    # Task spawning and lifecycle management
│   ├── isolation.rs     # Context/terminal session isolation per subagent
│   ├── aggregation.rs   # Result collection and merging
│   ├── timeout.rs       # Timeout management for child tasks
│   └── error.rs         # Error types
├── tests/
│   └── integration.rs   # End-to-end delegation tests
└── Cargo.toml
```

### Integration Points

1. **Conductor API Endpoint:** `POST /delegate` — submit delegation request
2. **CLI Subcommand:** `crabjar delegate --goal "..." --context "..."`
3. **Tool Registration:** Register in crabjar's tool registry for agent use

### Key Design Decisions

1. **Process Isolation:** Each subagent runs in a separate process with its own terminal session (via tmux or pty)
2. **Context Separation:** Subagents receive only the context explicitly passed to them — no shared conversation history
3. **Result Delivery:** For foreground delegation, block until all children complete; for background, deliver via callback or polling endpoint
4. **Concurrency Control:** Enforce `delegation.max_concurrent_children` limit (default 3)
5. **Timeout Management:** Per-task and aggregate timeout with configurable defaults

## Testing Strategy

1. **Unit tests:** Task isolation, result aggregation, timeout handling
2. **Integration tests:** Full delegation flow with real subagent spawning
3. **Hermes compatibility tests:** Same inputs produce equivalent outputs
4. **Edge cases:** Empty task list, single task, all tasks fail, partial failures

## Acceptance Criteria

- [ ] Tool schema matches Hermes (same parameter names, types)
- [ ] Single task delegation works: spawn, execute, return result
- [ ] Batch delegation works: multiple parallel subagents
- [ ] Background delegation returns immediately with task IDs
- [ ] Concurrency limit enforced
- [ ] Timeout management works for individual and aggregate tasks
- [ ] Result aggregation merges all child outputs correctly
- [ ] Error handling for partial failures
- [ ] Integration with guard system for authorization

## Success Metrics

- Users can migrate from Hermes to crabjar without losing delegation capability
- Delegation tool calls produce equivalent results to Hermes
- Parallel subagent execution provides measurable speedup for independent tasks
- All integration tests pass

## Effort Estimate

- **Complexity:** High (process isolation, concurrency, result aggregation)
- **Time:** 2-3 weeks
- **Risk:** Medium (complex interaction with conductor and guard systems)