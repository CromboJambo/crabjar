# herdr-gpui: GPU-Accelerated Herdr Client for Crabjar Integration

**Created:** 2026-09-24  
**Last Updated:** 2026-09-24  
**Status:** Fresh  
**Source:** https://github.com/penso/herdr-gpui  

## Overview

herdr-gpui is a native Rust/GPUI client for Herdr daemons that renders terminal surfaces without running another terminal emulator. It uses GPU-accelerated text rendering via the GPUI framework, providing smooth split-pane layouts and responsive interaction.

**Key characteristics:**
- 176 Rust source files across 4 crates (herdr-gpui, herdr-client, herdr-protocol, herdr-daemon)
- ~80K lines of Rust code
- Uses GPUI framework for GPU-accelerated rendering (similar to Zed editor's stack)
- Binary protocol over Unix sockets for client-daemon communication
- Supports macOS, Linux (x86_64/aarch64), experimental Windows support

## Architecture

```
herdr-gpui (GUI app)
  └── herdr-client (socket worker, session management)
      └── herdr-protocol (binary framing, surface patches)
          └── herdr daemon (terminal processes, workspaces, agents)
```

## Technical Deep-Dive

### Rendering Pipeline
- **GPUI framework:** Uses the same GPU-accelerated UI stack as Zed editor
- **Cell-based rendering:** Paints terminal cells directly, no intermediate TTY layer
- **Split panes:** Native support for multiple terminal surfaces in a single window
- **Performance:** Optimized for dense terminals (160x50 tested), with text-shaping caching

### Protocol Design
- **Binary framing:** Efficient serialization over Unix domain sockets
- **Surface patches:** Delta updates rather than full screen redraws
- **Event-driven:** Input events flow back through the same socket channel
- **Session management:** Multiple concurrent sessions per daemon instance

### Build System
- Uses `just` task runner with comprehensive CI/CD pipeline
- Supports Nix flake, Homebrew cask, and manual installation
- Cross-platform: macOS (Apple Silicon + Intel), Linux (glibc 2.39+)

## Integration Opportunities for Crabjar & Terrarium

### High-Value: GPU-Accelerated Agent Dashboard

**Current crabjar state:** Terminal-based UI via ratatui with limited concurrent view support.

**Opportunity:** Build a GPUI-based dashboard that displays multiple agent sessions simultaneously, similar to herdr-gpui's split-pane model. This would enable:
- Real-time monitoring of multiple crabjar agents in parallel
- GPU-accelerated rendering for smooth updates during high-throughput execution
- Better utilization of modern display hardware

**Implementation approach:**
1. Adapt crabjar's exec pipeline to emit structured surface patches (not raw terminal output)
2. Reuse herdr-protocol's binary framing for efficiency
3. Build a crabjar-gpui crate that connects to crabjar's orchestrator via adapted protocol
4. Maintain TUI as fallback for headless environments

### Medium-Value: Terrarium Visualization Enhancement

**Current terrarium state:** DAGR events with JSON-RPC interface, text-based status reporting.

**Opportunity:** Use herdr-gpui's rendering approach to visualize multiple terrarium zones simultaneously:
- Each zone/agent gets its own pane in a split layout
- Status changes trigger surface patches (color coding for states)
- Click-through interaction to select/focus specific agents
- Better debugging and monitoring of complex multi-agent scenarios

### Low-Value but Interesting: Protocol Compatibility Layer

**Opportunity:** Make crabjar sessions compatible with herdr-gpui as a client.
- Would require crabjar daemon to speak herdr protocol
- Enables users to view crabjar sessions through any herdr-compatible GUI
- Significant protocol translation work required

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| GPUI dependency adds build complexity | High | Medium | Keep TUI as default, GPUI as optional feature |
| Protocol adaptation is non-trivial | High | High | Start with read-only dashboard before bidirectional |
| GPU requirement excludes some environments | Low | Low | CPU fallback in GPUI handles this |

## Recommendations

1. **Immediate:** Study herdr-protocol crate for surface patch encoding patterns - applicable to crabjar's state update mechanism regardless of GUI choice.

2. **Short-term (Week 1-2):** Build proof-of-concept crabjar-gpui dashboard showing read-only agent states from orchestrator SSE stream. Validate performance characteristics.

3. **Medium-term (Week 3-4):** If POC successful, implement bidirectional communication allowing user to select/interact with agents through GPUI interface.

4. **Long-term:** Consider making herdr-gpui a first-class client option for crabjar deployments, similar to how TUI and CLI coexist currently.

## Structural Analysis Findings (pesti-structural-tokenizer)

**Repository scale:** 176 Rust source files, ~80,484 total lines across 4 crates.

### Tokenization Results

**herdr-client/src/connect.rs:** 136 tokens
- Heavy async stream usage with atomic state management
- Complex error handling with custom ConnectError type
- Demonstrates client-side protocol interaction pattern

**herdr-protocol/src/lib.rs:** 11 tokens (re-export facade)
- Clean module boundary exposing core types
- Follows crabjar's preferred "glass" abstraction pattern

### Protocol Layer Architecture

The herdr protocol uses a binary serialization approach with several key design patterns relevant to crabjar:

1. **Surface-based rendering model:** Terminal state represented as surfaces with patches rather than full redraws - directly applicable to agent state updates
2. **Delta update efficiency:** `PaneSurfacePatch` messages minimize bandwidth for frequent small changes
3. **Built-in agent status tracking:** Protocol already defines `Idle`, `Working`, `Blocked`, `Done`, `Unknown` states that map directly to crabjar's agent lifecycle

### Key Integration Insight

herdr-gpui's architecture is remarkably aligned with crabjar's needs:
- Split-pane layout naturally maps to multiple simultaneous agent executions
- Surface patching protocol reduces overhead for frequent agent status changes
- Binary framing could replace JSON/SSE for more efficient internal communication
- The daemon/client separation mirrors crabjar's orchestrator/agent model

## Updated Integration Strategy

Based on structural analysis, the integration path is clearer than initially assessed:

### Phase 1: Protocol Study (Week 1)
- Deep-dive into herdr-protocol binary framing patterns
- Map crabjar agent states to herdr's status enum
- Design crabjar-specific surface types for agent output/status/errors

### Phase 2: Read-Only Dashboard POC (Week 2-3) - COMPLETE
- [x] Build crabjar-gpui crate structure with Cargo.toml and dependencies
- [x] Design mock agent data model matching herdr's AgentStatus enum
- [x] Implement dashboard UI using GPUI framework patterns from herdr-gpui
- [x] Demonstrate split-pane agent visualization with mock data
- [x] Validate GPU rendering performance with synthetic load

**POC Artifacts:**
- `crates/crabjar-gpui/Cargo.toml` - crate manifest with gpui 0.2.2 dependency
- `crates/crabjar-gpui/src/main.rs` - 198-line dashboard with mock multi-agent data
- Workspace Cargo.toml updated to include crabjar-gpui member

### Phase 3: Live Integration (Week 4+)
- Connect to real crabjar orchestrator SSE stream
- Implement protocol translation layer
- Add bidirectional interaction capabilities

## Success Metrics

- **Performance:** 60fps rendering with <50 concurrent agent views
- **Efficiency:** <1ms latency for surface patch updates
- **Scalability:** Linear performance degradation as agent count increases
- **Compatibility:** Works on systems without GPU (CPU fallback)
