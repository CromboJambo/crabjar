**⚠️ Disclaimer: Experimental Project ⚠️**

Crabjar is an **experimental learning hobby project**. The codebase may contain architectural compromises, undocumented features, and deliberate use of unsafe Rust blocks for educational purposes. This repository is not intended for production use without thorough review and refactoring.

Crabjar is a Rust 2024 workspace centered on the `crabjar` CLI. It includes state-docs management, knowledge-store bridge, orchestrator (Axum SSE server), guard (execution gate with scope isolation and trust resolution), telemetry (flight recorder), sandbox (agent isolation), tool registry, skill crates, and a Zed Agent Protocol bridge. The ReAct agent loop lives in `host/host-agent/`. LLM inference tooling (runner, safetensors, GGUF parser) is in a separate `llm-workspace/`.

## Workspace Members

Declared in `Cargo.toml` `[workspace.members]`:

- `memory` — agent-context SQLite storage
- `guard` — trust layers, annealing, execution gate
- `telemetry` — flight recorder, command executor
- `orchestrator` — Axum SSE server (ACP orchestrator)
- `sandbox` — agent isolation tooling
- `tool_registry` — MCP tool registry
- `axum-mux` — Axum mux utilities
- `crabjar-architecture` — mechanical dependency boundary enforcement (8-layer model)
- `host/host-core` — core host runtime
- `host/host-system` — system observation
- `host/host-observe` — observation utilities
- `host/host-agent` — ReAct agent loop with model routing
- `host/host-webview` — webview host
- `host/host-mqtt` — MQTT host
- `host/host-graph` — graph utilities
- `host/host-screen` — screen capture
- `apps/teams` — teams plugin (reference application)
- `src/host-binary` — host binary
- `src/skill-script-runner` — skill script discovery and execution
- `src/skill-reference-store` — skill reference indexing and staleness
- `zed-acp-bridge` — Wasm extension (tool call mapping + gate enforcement)
- `zed-acp-server` — stdio JSON-RPC server (ACP protocol)

Nested crates under `src/` use `src/<crate>/src/` pattern (not flat `src/<crate>/`).

## Architecture

### Core crates

| Component | Role |
| :--- | :--- |
| `crabjar` (src/) | CLI — state-docs, knowledge, guard, exec, dotfile, doctor commands |
| `memory` | Agent-context SQLite storage + state-docs querier |
| `orchestrator` | Axum SSE server (ACP orchestrator) |
| `guard` | Trust layers, execution gate, scope isolation, fingerprint approvals |
| `telemetry` | Flight recorder |
| `sandbox` | Agent isolation |
| `tool_registry` | MCP tool registry with discovery |
| `crabjar-architecture` | Mechanical dependency boundary enforcement |
| `axum-mux` | Axum mux utilities |
| `host/host-agent` | ReAct agent loop with model routing, context compression |
| `host/host-core` | Core host runtime |
| `host/host-system` | System observation |
| `host/host-observe` | Observation utilities |
| `host/host-webview` | Webview host |
| `host/host-mqtt` | MQTT host |
| `host/host-graph` | Graph utilities |
| `host/host-screen` | Screen capture |
| `apps/teams` | Teams plugin (reference application) |
| `src/host-binary` | Host binary |
| `src/skill-script-runner` | Skill script discovery and execution |
| `src/skill-reference-store` | Skill reference indexing and staleness |
| `zed-acp-bridge` | Wasm extension (tool call mapping + gate enforcement) |
| `zed-acp-server` | stdio JSON-RPC server (ACP protocol) |

### File layout

```
crabjar/
├── Cargo.toml               # Workspace root
├── Cargo.lock               # Locked dependency graph
├── AGENTS.md               # Repository guidelines
├── README.md               # Project overview
├── ROADMAP.md              # Development priorities
├── project_map.md          # Structural map
├── agent_config.md         # Agent configuration
├── Justfile               # Task runner shortcuts
├── Containerfile, Dockerfile
│
│  Core crates
├── memory/                  # Agent-context SQLite storage
├── orchestrator/            # Axum SSE server
├── guard/                   # Trust layers, execution gate
├── telemetry/               # Flight recorder
├── sandbox/                 # Agent isolation
├── tool_registry/           # MCP tool registry
├── crabjar-architecture/    # Dependency boundary enforcement
├── axum-mux/               # Axum mux utilities
│
│  Host runtime
├── host/                    # 8 host crates
│   ├── host-core/
│   ├── host-system/
│   ├── host-observe/
│   ├── host-agent/          # ReAct agent loop
│   ├── host-webview/
│   ├── host-mqtt/
│   ├── host-graph/
│   └── host-screen/
│
│  Host apps
├── apps/teams/              # Teams plugin (reference app)
├── src/host-binary/         # Host binary
│
│  Skill crates
├── src/skill-script-runner/ # Skill script discovery
├── src/skill-reference-store/ # Skill reference indexing
│
│  Zed ACP
├── zed-acp-bridge/          # Wasm extension
├── zed-acp-server/          # stdio JSON-RPC server
│
│  CLI source
├── src/main.rs              # CLI entry point
├── src/lib.rs               # shared library surface
├── src/knowledge_store/     # Knowledge store logic
├── src/bitwarden/           # Bitwarden integration
├── src/metrics/             # Metrics (module sizes, test count)
├── src/doctor.rs            # Environment health checks
├── src/dotfile_manager.rs   # Dotfile management
├── src/project_loader.rs    # Project loading
├── src/tool_registry_cli.rs # Tool registry CLI
│
│  State docs
├── state-docs/              # Durable Markdown state documentation
├── state-docs/overlay/      # Overlay JSON sidecars
│
│  Other
├── testing/configs/         # Test configs
├── tests/cli.rs             # CLI integration tests
├── crabjar-skills/          # Skill documentation templates
├── ui-state-copy/           # UI state copy
└── scripts/                 # Helper scripts
```

## Build & Test

Use `just` for workflows:

- `just check`: `cargo check --workspace`
- `just build`: `cargo build -p crabjar`
- `just run state list`: `cargo run -p crabjar -- state list` (args replaceable)
- `just test`: `cargo test --workspace`
- `just clean`: removes build artifacts

Narrow scope: `cargo check/clippy/test -p <crate>`

Formatting: `cargo fmt --all`

Lint: `cargo clippy --workspace -- -D warnings`

## CLI Commands

### State docs

```bash
crabjar state list
crabjar state show <doc>
crabjar state annotate <doc> <message>
crabjar state question <doc> <message>
crabjar state resolve <doc> <id>
crabjar workspace status
```

### Knowledge

```bash
crabjar knowledge sync <doc>
crabjar knowledge query --tags=<tag>
crabjar knowledge events --limit=<n>
crabjar knowledge verify
crabjar knowledge deactivate <id> --reason=<reason>
```

### Guard

```bash
crabjar guard queue
crabjar guard approve <id>
crabjar guard reject <id>
crabjar guard resolution <action>
```

### Exec

```bash
crabjar exec <action>
```

### Other

```bash
crabjar dotfile <command>
crabjar doctor check
crabjar backend list
crabjar metrics module-sizes
crabjar metrics test-count
crabjar tool list
crabjar tool discover
```

## State Docs

Crabjar treats `state-docs/` as a shared project memory surface. Markdown files are the durable source documents; agent/user comments live in `state-docs/overlay/*.overlay.json` so they can be updated without rewriting the base docs.

## Acknowledgments

Crabjar's security and execution model draws from [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw):
- **Guard/trust/annealing** — inspired by ZeroClaw's six-layer security model and autonomy levels
- **Tool receipts** — HMAC-SHA256 per-invocation receipts
- **Microkernel design** — trait-based kernel ABI, feature-flagged subsystems
- **SOP engine** — declarative procedures with approval gates
- **Config schema generation** — `schemars`-driven docs from live schema

Crabjar's contribution: dynamic confidence tracking + trust evolution (annealing).

Crabjar's authorization-order contract draws from [CodeWhale](https://github.com/Hmbown/CodeWhale):
- **Monotonic authorization order** — a later safety layer can only tighten, never loosen, an earlier block/hold; pinned by regression tests in `guard/src/authorization_order.rs`
- **Fleet identity/selection contract** — stable member id + semantic role + exact model identity, with ambiguity errors that name candidates (pattern to adopt for apps/teams)
- **Deterministic task scorers** — `exit_code` / `file_exists` / `regex_match` / `json_path` verifiers with typed receipts (pattern for ephemeral-VM task verdicts)
- **Roster-doesn't-execute separation** — the fleet layer resolves membership only; execution runs under the runtime's policy

CodeWhale's contribution: single-machine terminal coding agent with a multi-worker Fleet layer, OS sandboxing, and MCP/hooks/skills — a worker-shaped peer to crabjar's environment + trust layer.
Crabjar's contribution: trust layers, annealing, provenance chains, ephemeral VM environment management, and state-docs — crabjar is the substrate CodeWhale-style workers would run on, not a replacement for them.

## Version

0.12.0

## Repository

https://github.com/crombojambo/crabjar
