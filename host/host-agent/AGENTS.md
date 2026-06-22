# AGENTS.md — crabjar-host-agent (host-agent)

> Purpose: Agent loop — observe → understand → plan → execute → verify → reflect cycle.

## Layer

Layer 4: host — host runtime crates, may depend on layers 0, 1, 2, 3, 4.

## Public API

- Agent loop state machine
- Context compression between turns
- Model routing (which model for which phase)
- Decision flow: when to call tools vs. respond directly

## Key Files

- `src/lib.rs` — crate entry point
- `src/agent_loop.rs` — ReAct loop implementation
- `src/context.rs` — context management
- `src/model_routing.rs` — model selection logic

## Dependencies

- tokio, serde, serde_json, tracing, uuid, chrono, thiserror, async-trait, rusqlite, reqwest, tempfile, crabjar-host-core, crabjar-host-observe

## Pitfalls

- Context compression is critical for long conversations — balance loss vs. retention
- Model routing decisions should be explicit and traceable
- The agent loop is the core data flow — changes here ripple across all host crates
- Tool invocation requires guard authorization before execution
