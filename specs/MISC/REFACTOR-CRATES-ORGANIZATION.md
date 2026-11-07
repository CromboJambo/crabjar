# Crabjar Crate Organization Refactor Spec

**Status:** In Progress  
**Last Updated:** 2026-11-05  
**Author:** crabjar dev team  

## Overview

This spec defines the target crate organization for the crabjar workspace. The goal is to establish a clean, consistent directory structure that makes it easy to navigate, understand dependencies, and maintain the project as it grows.

## Current State (Post-Initial Refactor)

### Completed Reorganizations

1. **All `crabjar-*` crates** moved to `crates/crabjar/` (45 packages)
2. **All `analyze-*` crates** moved to `crates/analyze/` (8 packages)
3. **VM crates** grouped in `vm/` (vm-core, axum-mux/vm-bridge)
4. **Host crates** grouped in `host/` (8 packages: core, system, observe, agent, webview, mqtt, graph, screen)
5. **Apps** in `apps/teams/` (crabjar-app-teams)
6. **Loose root crates** moved to `crates/`: benchmarks, scheduler, slow-friend, universal-structure, zed-acp-bridge, zed-acp-server, email-smtp, lab-topology, podman, ssh-bastion, terminal, train-dedup, train-extract, train-filewatcher, train-secret-stripper
7. **Documentation** consolidated into `docs/` library
8. **src/ nested crates** moved to `crates/crabjar/`: crabjar-host, skill-script-runner, skill-reference-store, skill-process-management, hermes-struct-tok-pipeline, skill-structural-tokenizer

### Remaining Issues

1. **Root-level standalone crates**: `guard`, `memory`, `orchestrator`, `sandbox`, `telemetry` are still at the project root rather than in `crates/`
2. **Training pipeline crates** should be grouped under a dedicated directory
3. **No clear grouping** for some related crates (e.g., zed-acp-* could be grouped)

## Target Structure

```
crabjar/
├── Cargo.toml              # Root package: crabjar CLI binary + lib
├── src/                    # Main crabjar CLI source code only
│   ├── main.rs
│   ├── lib.rs
│   ├── commands/           # CLI command modules
│   ├── bitwarden/          # Bitwarden integration module
│   ├── crabjar_config/     # Config module
│   ├── habitat_contract/   # Habitat contract module
│   ├── knowledge_store/    # Knowledge store module
│   └── metrics/            # Metrics module
├── crates/                 # All library crates
│   ├── crabjar/            # Core crabjar crates (45 packages)
│   │   ├── crabjar-analyze/
│   │   ├── crabjar-browser/
│   │   ├── ...
│   ├── analyze/            # Analysis toolkit (8 packages)
│   │   ├── analyze-apk/
│   │   ├── analyze-common/
│   │   ├── ...
│   ├── train/              # Training pipeline (4 packages)
│   │   ├── train-dedup/
│   │   ├── train-extract/
│   │   ├── train-filewatcher/
│   │   └── train-secret-stripper/
│   ├── acp/                # Agent Communication Protocol (2 packages)
│   │   ├── zed-acp-bridge/
│   │   └── zed-acp-server/
│   ├── benchmarks/
│   ├── email-smtp/
│   ├── lab-topology/
│   ├── podman/
│   ├── scheduler/
│   ├── slow-friend/
│   ├── ssh-bastion/
│   ├── terminal/
│   └── universal-structure/
├── guard/                  # Execution gate, trust layers (standalone)
├── host/                   # Host integration crates (8 packages)
│   ├── host-core/
│   ├── host-system/
│   ├── ...
├── memory/                 # Agent-context SQLite storage (standalone)
├── orchestrator/           # Axum SSE server + inference backend (standalone)
├── sandbox/                # Sandboxing infrastructure (standalone)
├── telemetry/              # Telemetry collection (standalone)
├── vm/                     # VM lifecycle management (2 packages)
│   ├── vm-core/
│   └── axum-mux/
├── apps/                   # Reference applications
│   └── teams/
└── docs/                   # Documentation library
```

## Migration Plan

### Phase 1: Move Root-Level Crates to crates/

**Goal:** All library crates live under `crates/`. Only the main CLI package (`crabjar`) remains at root.

| Crate | Current Location | Target Location | Dependencies to Update |
|-------|-----------------|-----------------|----------------------|
| guard | `/guard` | `crates/guard` | Many crates depend on this |
| memory | `/memory` | `crates/memory` | Several crabjar-* crates |
| orchestrator | `/orchestrator` | `crates/orchestrator` | Depends on many internal crates |
| sandbox | `/sandbox` | `crates/sandbox` | crabjar-worker, others |
| telemetry | `/telemetry` | `crates/telemetry` | Multiple crates |

**Steps:**
1. For each crate: `mv <crate> crates/<crate>`
2. Update workspace members in root `Cargo.toml`
3. Update all path dependencies referencing the moved crate
4. Run `cargo check --workspace` to verify
5. Commit after each successful move

### Phase 2: Group Training Pipeline Crates

**Goal:** Consolidate training-related crates under a dedicated directory.

| Crate | Current Location | Target Location |
|-------|-----------------|-----------------|
| train-dedup | `crates/train-dedup` | `crates/train/train-dedup` |
| train-extract | `crates/train-extract` | `crates/train/train-extract` |
| train-filewatcher | `crates/train-filewatcher` | `crates/train/train-filewatcher` |
| train-secret-stripper | `crates/train-secret-stripper` | `crates/train/train-secret-stripper` |

**Steps:**
1. Create `crates/train/` directory
2. Move each training crate into it
3. Update workspace members and path dependencies
4. Verify build

### Phase 3: Group ACP Crates (Optional)

**Goal:** Consolidate Agent Communication Protocol crates.

| Crate | Current Location | Target Location |
|-------|-----------------|-----------------|
| zed-acp-bridge | `crates/zed-acp-bridge` | `crates/acp/zed-acp-bridge` |
| zed-acp-server | `crates/zed-acp-server` | `crates/acp/zed-acp-server` |

**Steps:**
1. Create `crates/acp/` directory
2. Move both crates into it
3. Update workspace members and path dependencies
4. Verify build

## Verification Strategy

After each phase:
1. `cargo check --workspace` — ensure all packages compile
2. `cargo test --workspace` — run full test suite
3. `just module-sizes-check` — verify no modules exceed 500 LoC
4. Update `docs/project_map.md` to reflect new structure

## Risks and Mitigations

### Risk: Breaking path dependencies
**Mitigation:** Use sed to systematically update all path references after each move. Run `cargo check` immediately after updates.

### Risk: CI failures due to stale paths in workflows
**Mitigation:** Check `.github/workflows/` for any hardcoded paths and update them.

### Risk: Documentation drift
**Mitigation:** Update `docs/project_map.md`, `AGENTS.md`, and relevant skills after each phase.

## Success Criteria

1. All library crates are under `crates/` (no root-level crates except main CLI)
2. Related crates are logically grouped in subdirectories
3. Workspace builds and tests pass
4. Documentation reflects the new structure
5. No hardcoded paths remain (use environment variables as established)

## Notes

- The main `crabjar` CLI package stays at root (`src/`) — this is conventional for Rust projects with a primary binary
- Grouping should be done by logical relationship, not just naming convention
- Each move should be committed separately to enable easy rollback if needed