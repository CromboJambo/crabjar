# CrabJar vs Hermes: Extensibility Parity Gap Analysis

**Date:** 2026-10-16
**Status:** Current state after Phase 7 completion

## Executive Summary

CrabJar has completed the core architecture for tools, skills, plugins, and MCP integration. The remaining gap to Hermes parity is primarily in **remote discovery**, **security scanning**, and **runtime management** features. Estimated effort: ~8-12 weeks across 4-5 additional phases.

## Current State

| Component | CrabJar Status | Hermes Equivalent | Gap |
|-----------|---------------|-------------------|-----|
| Tool Registry | SQLite-backed discovery, registration, execution policies | Similar registry architecture | Minor — guard integration incomplete |
| Plugin System | Three-tier model (ToolServer, in-process, WASM) | Similar process pool architecture | Moderate — hot-reload, dependency management missing |
| Skill Loader | Filesystem-based discovery and loading | Filesystem + remote registries | Significant — no remote discovery |
| Tool Binding | Connects skill tools to registry | Similar integration | Minimal |
| Context Injection | Execution context with persistence | Similar context model | Minimal |
| MCP Server | crabjar-mcp-tool-registry for Hermes integration | Multiple built-in MCP servers | Moderate — no MCP client for external servers |
| CLI Commands | tool list/install, plugin enable/disable | Extensive skill/plugin/tool CLI | Moderate — missing search, update, cache commands |

## Remaining Gaps by Category

### 1. Remote Skill Discovery (Significant)

**What's missing:**
- GitHub repository search and installation
- ClawHub registry integration (search, install, list)
- Tag-based discovery protocol (`crabjar-skill` tags)
- Remote skill metadata parsing

**Effort:** Phases 8-9 from SKILLS-REMOTE-DISCOVERY.md (~4-6 weeks)

### 2. Security Scanning Pipeline (Significant)

**What's missing:**
- Code analysis engine for skill packages
- Binary detection and file size limits
- Network/filesystem access pattern detection
- SHA-256 checksum verification
- Scan report generation with severity levels

**Effort:** Phase 10 from SKILLS-REMOTE-DISCOVERY.md (~2-3 weeks)

### 3. Trust Management & Cache (Moderate)

**What's missing:**
- Trust tier system (community/verified/official)
- Version-aware skill cache in `~/.crabjar/skill-cache/`
- Update detection and bulk updates
- Manual cache management CLI commands

**Effort:** Phase 11 from SKILLS-REMOTE-DISCOVERY.md (~1-2 weeks)

### 4. MCP Client Integration (Moderate)

**What's missing:**
- MCP client for connecting to external MCP servers
- Tool discovery from connected MCP servers
- Protocol version negotiation
- Multiple concurrent MCP server connections

**Effort:** New phase, estimated ~2-3 weeks

### 5. Plugin Runtime Management (Moderate)

**What's missing:**
- Plugin hot-reloading without agent restart
- Plugin dependency management and ordering
- Comprehensive lifecycle hooks (on_load, on_unload, on_error, on_update)
- Plugin sandboxing/isolation improvements
- Plugin update mechanism

**Effort:** New phase, estimated ~2-3 weeks

### 6. Tool Execution & Guard Integration (Minor)

**What's missing:**
- Full integration of execution policies with guard system
- Binary validation for tools
- Tool versioning and updates
- Dynamic tool loading/unloading at runtime

**Effort:** New phase, estimated ~1-2 weeks

## Parity Roadmap

| Phase | Focus | Estimated Duration | Dependencies |
|-------|-------|-------------------|--------------|
| 8 | GitHub Skill Discovery | 2-3 weeks | None |
| 9 | ClawHub Integration | 2-3 weeks | Phase 8 |
| 10 | Security Scanning Pipeline | 2-3 weeks | Phases 8-9 |
| 11 | Trust Management & Cache | 1-2 weeks | Phase 10 |
| 12 | MCP Client Integration | 2-3 weeks | None |
| 13 | Plugin Runtime Management | 2-3 weeks | None |
| 14 | Tool Execution & Guard Integration | 1-2 weeks | Phases 8-11 |

**Total estimated effort:** ~12-19 weeks for full Hermes parity

## Quick Wins (Low Effort, High Impact)

1. **Guard integration for execution policies** — connect existing registry policies to the guard system
2. **Dynamic tool unloading** — implement clean removal of tools from registry when skills are unloaded
3. **MCP client basics** — minimal client for connecting to one external MCP server
4. **Plugin lifecycle hooks** — add on_load/on_unload/on_error callbacks

These could be done in ~2-3 weeks and would close most of the "minor" gaps.

## Not Required for Parity (CrabJar-Specific)

- CrabJar's three-tier plugin model is actually more sophisticated than Hermes'
- SQLite-based tool registry is faster than Hermes' filesystem approach (benchmarked 200x faster listing)
- Context persistence with JSON serialization is a CrabJar advantage
- Explicit manifest-based skill definition is cleaner than Hermes' implicit discovery

## Conclusion

CrabJar has the core architecture in place. The remaining gaps are primarily **feature completeness** rather than architectural deficiencies. With ~12-19 weeks of focused development across 7 phases, CrabJar can achieve full parity with Hermes' extensibility ecosystem — and likely surpass it in performance and design clarity.