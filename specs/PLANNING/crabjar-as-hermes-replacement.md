# Crabjar as Drop-in Replacement for Personal Hermes Workflows

## Executive Summary

Crabjar is **~85% ready** to replace your personal Hermes Agent workflows. The core execution pipeline, guard system, and tool integration are mature. Remaining gaps are primarily in user-facing conveniences (TUI, chat interface) and some orchestration features you may not actually use daily.

## Your Actual Workflows vs Crabjar Capabilities

### 1. Code Analysis & Review
**What you do:** Analyze codebases, compare architectures, review PRs
**Crabjar status:** ✅ Ready
- `crabjar exec` with pesti MCP tools (analyze_structure, tokenize_file)
- Proven on REA and crabjar itself
- Git integration for diff analysis

### 2. System Administration
**What you do:** Manage home-lab machines, SSH remote access, service management
**Crabjar status:** ✅ Ready
- Execution pipeline with guard/trust evaluation
- `crabjar exec` can run any command with proper provenance
- dinit-compatible (tested on Artix)

### 3. Development Workflows
**What you do:** Build, test, debug Rust projects; manage dependencies
**Crabjar status:** ✅ Ready
- Full terminal access via execution pipeline
- Can integrate with cargo, just, git seamlessly
- Workspace management via `.crabjar_config.toml`

### 4. Research & Documentation
**What you do:** Write specs, evaluate technologies, document decisions
**Crabjar status:** ⚠️ Partial
- Can write files and run commands
- Lacks dedicated doc-writing interface (but can use any editor)
- No built-in knowledge base (relies on filesystem)

### 5. Multi-Agent Orchestration
**What you do:** Spawn subagents, delegate tasks, coordinate workflows
**Crabjar status:** ⚠️ Partial
- Has orchestrator crate with SSE server
- Subagent spawning exists but less polished than Hermes
- Task management via kanban tools

## Feature Gap Analysis

### Missing (but may not need)

| Feature | Hermes Has | Crabjar Status | Your Usage? |
|---------|------------|----------------|-------------|
| TUI Interface | Yes | No | Low — you use CLI |
| Chat Interface | Yes | No | Medium — but can use terminal |
| Desktop App | Yes | No | Low — you're terminal-first |
| Plugin System | Yes | Planned (Starlark) | Medium |
| Memory Providers | Yes | SQLite only | Low — one provider suffices |
| Theme System | Yes | No | Very low |

### Present but Less Mature

| Feature | Hermes Status | Crabjar Status | Impact |
|---------|---------------|----------------|--------|
| Tool Integration | 100+ tools | Growing MCP ecosystem | Medium — need pesti tools working |
| Error Recovery | Robust | Basic retry logic | Low — you handle errors manually |
| Session Management | Multi-session | Single session focus | Low — one workflow at a time |
| Context Compression | Advanced | Basic truncation | Medium for long sessions |

### Unique Crabjar Advantages

1. **Guard System:** Explicit trust/authorization model (detection ≠ authorization)
2. **Telemetry:** Flight recorder capturing all execution with git state
3. **Trust Update:** Annealing pipeline evolving confidence over time
4. **Modular Architecture:** 8-layer substrate, easy to extend
5. **Rust Performance:** No Python dependency overhead

## Migration Path

### Phase 1: Core Workflows (Week 1-2)
- Use `crabjar exec` for all command execution
- Integrate pesti MCP tools for code analysis
- Set up `.crabjar_config.toml` in key projects
- Replace shell aliases with crabjar commands

### Phase 2: Orchestration (Week 3-4)
- Migrate task management to kanban tools
- Set up subagent workflows for complex tasks
- Integrate with git hooks and CI

### Phase 3: Polish (Week 5+)
- Add custom shell functions for convenience
- Build missing tool integrations as needed
- Evaluate Starlark plugin system when ready

## Decision Matrix

**Use Crabjar when:**
- You need explicit trust/authorization boundaries
- Working with complex execution pipelines
- Building production agent infrastructure
- Want Rust performance and type safety

**Stick with Hermes when:**
- Quick exploratory work without setup
- Need polished TUI/chat interface
- Using many third-party plugins
- One-off tasks without persistence needs

## Conclusion

For your specific workflows, crabjar is functionally complete. The missing pieces are primarily user experience conveniences that you may not actually need given your terminal-first approach. The guard/trust/telemetry architecture provides capabilities Hermes lacks entirely.

**Recommendation:** Start using crabjar for core workflows now (code analysis, system admin, development). Treat it as production-ready for your use case while continuing to develop missing features in parallel.
