# crabjar

An execution framework where agents calibrate their own confidence, test their assumptions against reality, and build knowledge through relationships rather than flat assertions.

Multi-agent orchestration with execution gating, trust layers, and knowledge compression. Full tool parity with Hermes Agent — 25/25 core tools implemented natively in Rust.

## The Idea

Agents make mistakes. They're overconfident when they should be uncertain. Current systems either trust everything the agent says or require human review for every action — neither scales.

Crabjar measures confidence empirically:
- **Trust layers** (0-3) are earned through execution history, not declared by the agent
- **Annealing** decays confidence over time — untested knowledge rots
- You only notice calibration failures when they're catastrophic. The system is designed to surface those.

The best way to understand a system is to push it past its limits and see where it breaks. That's how we approach performance — benchmark everything, trust nothing until measured. Risk assessment adds ~30ns overhead. We know that because we tested it, not because we assumed it.

Knowledge isn't facts stored in rows. It's relationships between things. Crabjar stores knowledge as a graph — nodes (facts, patterns, rules) connected by edges (supports, contradicts, derived_from). You find what matters by following connections, not searching keywords.

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
| `crates/guard/` | Execution gate, trust layers, scope isolation |
| `crates/memory/` | Agent-context SQLite storage + state-docs querier |
| `crates/orchestrator/` | Axum SSE server (ACP orchestrator) + tool dispatch |
| `crates/crabjar/` | Capability crates: conductor, vision, exec, browser-vault, delegation, etc. |
| `host/` | Host applications and ReAct agent loop |
| `apps/` | Standalone applications (teams, terrarium) |
| `mcps/` | MCP servers (bitwarden, kdeconnect, ssh-file) |
| `state-docs/` | Durable Markdown state documentation |

## Documentation

- **[AGENTS.md](./AGENTS.md)** — Repository guidelines, coding style, conventions
- **[project_map.md](./project_map.md)** — Structural map of all workspace members
- **[ROADMAP.md](./ROADMAP.md)** — Development priorities and completed phases
- **[specs/](./specs/)** — Architecture decision records and specifications

### User Documentation

- **[CLI Workflows Guide](docs/user/cli-workflows.md)** — Day-to-day command-line operations
- **[TUI Guide](docs/user/tui-guide.md)** — Interactive terminal interface usage

### API & Integration

- **[Conductor HTTP API Reference](docs/api/conductor.md)** — Complete API endpoint documentation

### Deployment

- **[Fleet Coordination Deployment](docs/deployment/fleet-coordination.md)** — Multi-node deployment guide

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

The chicken-and-egg problem: how can an agent self-correct if it can't identify "self" outside its own context? Crabjar's answer is to build the external observer into the system — annealing, trust layer transitions, outcome tracking are mechanisms for the system to observe itself from outside. Not perfect. A compromise between true external observation (impossible) and blind self-trust (dangerous).

## Version

0.13.0

## Repository

https://github.com/crombojambo/crabjar
---

**Contact**: [projects@crombo.dev](mailto:projects@crombo.dev) · **Security**: [Private vulnerability reporting](https://github.com/CromboJambo/crabjar/security/advisories/new)