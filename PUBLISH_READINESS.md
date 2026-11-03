# Crabjar Publish Readiness Assessment

**Date:** 2026-11-03
**Purpose:** Assess crates.io publishing readiness for crabjar workspace packages

## Current State

The crabjar workspace contains ~80 Cargo.toml files. Most internal components are explicitly marked `publish = false`. The following packages are potentially publishable:

### Main Package
| Package | Version | Status | Notes |
|---------|---------|--------|-------|
| crabjar | 0.12.4 | Ready? | Main CLI binary; depends on many internal crates |

### Internal (publish = false) — 12 packages
These are explicitly marked as not publishable:
- orchestrator, guard, telemetry, sandbox, tool_registry
- zed-acp-bridge, zed-acp-server
- memory (agent-context), crabjar-delegation
- skill-script-runner, skill-reference-store, skill-process-management

### Potentially Publishable — Need Assessment
| Package | Version | Notes |
|---------|---------|-------|
| crabjar-host | 0.12.4 | Host binary |
| vm-bridge | 0.1.0 | VM bridge service |
| terrarium | 0.1.0 | Isometric habitat app |
| crabjar-terminal | — | Terminal UI components |
| crabjar-commit / crabjar-commit-cli | 0.1.0 | Commit tooling |
| crabjar-gpui | 0.1.0 | GPU dashboard |
| crabjar-kanban-orchestrator | 0.1.0 | Kanban integration |
| crabjar-podman | 0.1.0 | Podman integration |
| hermes-struct-tok-pipeline | 0.1.0 | Structural tokenization pipeline |

## Publish Strategy

**Decision needed:** Should crabjar packages be published to crates.io, or kept as private/internal?

If publishing:
1. Main `crabjar` package depends on many internal crates — need to publish dependencies first
2. Check for proper metadata (license, description, homepage) in each Cargo.toml
3. Verify no private path dependencies remain

## Notes

- pesti crates (pesti-gguf, pesti-runner, etc.) are in a separate project, not this workspace
- This assessment supersedes the previous PUBLISH_READINESS.md from 2026-10-18 which referenced the old pesti workspace structure
