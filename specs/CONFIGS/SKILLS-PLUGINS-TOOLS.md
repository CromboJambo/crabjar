# Crabjar Skills, Plugins, and Tools Architecture

**Status:** Draft — locked for implementation planning
**Date:** 2026-10-15
**Author:** Hermes Agent (with user review)

## Overview

Crabjar's extensibility ecosystem consists of three layers that work together:

1. **Tools** — discrete capabilities with JSON schemas, execution policies, and trust layers
2. **Plugins** — process-isolated extensions with lifecycle management
3. **Skills** — bundles of tools + knowledge + procedures loaded from filesystem

This document defines the architecture for building out the unified skill system that ties these layers together.

## Current State

### Existing Components

| Component | Location | Status |
|-----------|----------|--------|
| Tool Registry | `tool_registry/` | SQLite-backed tool discovery, registration, execution policies, trust layers, binary validation |
| Plugin System | `crabjar-plugin/` | Three-tier model (ToolServer subprocesses, in-process scripts, WASM), process pool with health checks and lifecycle management |
| Skill Crates | `src/skill-*` | Four separate crates: script-runner, reference-store, terminal, web-fetch |

### What's Missing

A unified skill system that:
- Loads skills from filesystem/directories
- Injects execution context to loaded skills
- Binds skill tools to the tool registry
- Provides dynamic skill discovery and activation

## Architecture

```
crabjar-agent
├── Tool Registry (existing) — SQLite-backed tool discovery/registration
├── Plugin Manager (existing) — process pool for subprocess plugins
└── Skill System (NEW) — skill loading, context injection, tool binding
    ├── Skill Loader — discovers skills from filesystem/directories
    ├── Skill Context — provides execution context to loaded skills
    └── Tool Binder — connects skill tools to the registry
```

## Phase 1: Skill Loading Infrastructure

**Goal:** Create `crabjar-skills` crate with skill discovery and loading.

### Tasks

1. **Create `crabjar-skills` crate**
   - Add to Cargo workspace
   - Define crate structure (lib.rs, modules)
   - Establish dependencies on existing crates (tool_registry, crabjar-plugin)

2. **Implement Skill Loader**
   - Filesystem-based skill discovery (scan directories for skill manifests)
   - Parse skill manifests (JSON/YAML) to extract:
     - Tool definitions
     - Dependencies
     - Context requirements
   - Support both local and remote skills (URLs)

3. **Define Skill Manifest Schema**
   - Standardize manifest format across all skills
   - Include fields for version, author, description, tools, dependencies
   - Validate manifests against schema

### Acceptance Criteria

- Can discover skills from a specified directory
- Can parse and validate skill manifests
- Can list discovered skills with their tool definitions
- Unit tests for loader functionality

## Phase 2: Tool Binding

**Goal:** Connect loaded skill tools to the existing tool registry.

### Tasks

1. **Implement Tool Binder**
   - Register skill tools with the tool registry
   - Map tool names to execution handlers
   - Handle name collisions between skills

2. **Execution Policy Evaluation**
   - Evaluate trust layers for skill tools
   - Apply execution policies (low_risk, medium_risk, high_risk)
   - Integrate with guard system for authorization

3. **Dynamic Tool Loading/Unloading**
   - Support loading/unloading skills at runtime
   - Update tool registry accordingly
   - Handle cleanup and resource management

### Acceptance Criteria

- Skill tools appear in tool registry after loading
- Can execute skill tools through the registry
- Execution policies are enforced for skill tools
- Can unload skills and remove their tools from registry

## Phase 3: Context Injection

**Goal:** Provide skills with execution context and support dynamic updates.

### Tasks

1. **Skill Context Structure**
   - Define context fields (cwd, environment, session state)
   - Support skill-specific configuration overrides
   - Provide read/write access to context from skills

2. **Dynamic Context Updates**
   - Update context during execution
   - Notify loaded skills of context changes
   - Handle context serialization/deserialization

3. **Context Persistence**
   - Save context state between sessions
   - Load persisted context on startup
   - Support context versioning and migration

### Acceptance Criteria

- Skills can read execution context
- Context can be updated dynamically
- Context persists between sessions
- Skills can override default context values

## Design Decisions

1. **Pure Rust where possible** — use existing crates (`serde`, `tokio`, `rusqlite`) rather than external dependencies
2. **Explicit over implicit** — skills declare their tools and requirements in manifests, no magic discovery
3. **Incremental adoption** — new skill system coexists with existing skill crates during transition
4. **Backward compatibility** — existing tool registry and plugin system remain unchanged

## Risks

- **Complexity:** Adding a skill loading layer may introduce unnecessary indirection
  - Mitigation: Keep the loader simple; focus on filesystem-based skills first
- **Performance:** Dynamic tool loading/unloading may impact runtime performance
  - Mitigation: Profile early and optimize hot paths
- **Compatibility:** Existing skill crates may not fit the new manifest schema
  - Mitigation: Provide adapter layer for legacy skills

## Timeline

- Phase 1: 2-3 weeks (skill loader, manifest parsing)
- Phase 2: 2-3 weeks (tool binding, execution policies)
- Phase 3: 2-3 weeks (context injection, persistence)
- Total: ~6-9 weeks for complete implementation

## Success Metrics

- [ ] Skill loader discovers skills from filesystem
- [ ] Skill tools registered with tool registry
- [ ] Execution policies enforced for skill tools
- [ ] Context injected into loaded skills
- [ ] Dynamic loading/unloading of skills at runtime
- [ ] Integration tests pass across all phases
- [ ] Documentation updated for new skill system
