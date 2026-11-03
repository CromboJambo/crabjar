# crabjar

Multi-agent orchestration framework with execution gating, trust layers, and knowledge compression. Drop-in replacement for Hermes Agent.

## Quick Start

```bash
just build          # cargo build -p crabjar
just run state list # cargo run -p crabjar -- state list
just test           # cargo test --workspace
```

Use `just` for all workflows — see [Justfile](./Justfile).

## Key Directories

| Path | Purpose |
|------|---------|
| `src/` | CLI entry point + shared library surface |
| `memory/` | Agent-context SQLite storage + state-docs querier |
| `guard/` | Execution gate, trust layers, scope isolation |
| `orchestrator/` | Axum SSE server (ACP orchestrator) |
| `host/host-agent/` | ReAct agent loop with model routing |
| `crabjar-conductor/` | Fleet coordination layer |
| `apps/` | Standalone host applications (teams, terrarium) |
| `state-docs/` | Durable Markdown state documentation |

## Documentation

- **[AGENTS.md](./AGENTS.md)** — Repository guidelines, coding style, conventions
- **[project_map.md](./project_map.md)** — Structural map of all workspace members
- **[ROADMAP.md](./ROADMAP.md)** — Development priorities and completed phases
- **[specs/](./specs/)** — Architecture decision records and specifications

## Build & Test

```bash
just check          # cargo check --workspace
just build          # cargo build -p crabjar
just test           # cargo test --workspace
just clippy         # cargo clippy --workspace -- -D warnings
just fmt            # cargo fmt --all
```

Narrow scope: `cargo check/clippy/test -p <crate>`

## CLI Commands

- `crabjar state list|show|annotate` — State docs management
- `crabjar knowledge sync|query|verify` — Knowledge store operations
- `crabjar guard queue|approve|reject` — Execution gate approvals
- `crabjar exec <action>` — Execute actions through the pipeline
- `crabjar tool list|search|install` — MCP tool registry

Run `crabjar --help` for complete command reference.

## Architecture

8-layer dependency model enforced by `crabjar-architecture`. The glass separates stable substrate (guard, host-core) from disposable integrations. See [AGENTS.md](./AGENTS.md) for the full principle.

## Version

0.12.4

## Repository

https://github.com/crombojambo/crabjar