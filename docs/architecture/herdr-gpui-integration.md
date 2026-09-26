# Crabjar + Herdr-GPUI Integration Architecture

## Overview
This document defines the architecture for integrating herdr-gpui's GPU-accelerated rendering with crabjar's agent orchestration system to provide a high-performance multi-agent dashboard.

## Protocol Compatibility Analysis

The herdr protocol (crates/herdr-protocol) uses binary serialization via bincode with positional encoding. The key message types relevant to crabjar integration are:

### Agent Status Mapping
Herdr's `AgentStatus` enum maps directly to crabjar's agent lifecycle:
- `Idle` → Crabjar agent waiting for work
- `Working` → Crabjar agent executing task
- `Blocked` → Crabjar agent awaiting dependency/input
- `Done` → Crabjar agent completed successfully
- `Unknown` → Crabjar agent state unknown/error

### Surface Rendering Model
Herdr uses a surface-based rendering approach with delta updates:
- Full frame: `PaneSurfaceFrame` (initial render)
- Delta patches: `PaneSurfacePatch` (incremental updates)
- Each patch contains row-level cell changes only

This is highly efficient for agent dashboards where most of the UI remains static between status updates.

## Integration Architecture

```
crabjar orchestrator
    │
    ├── SSE stream (existing) ──► crabjar TUI (ratatui)
    │
    └── [NEW] CrabjarSurfaceAdapter ──► herdr protocol messages ──► crabjar-gpui client
            (translates agent events to surface patches)
```

### Component: CrabjarSurfaceAdapter
Location: `crates/crabjar-gpui/src/adapter.rs`

Responsibilities:
1. Subscribe to crabjar orchestrator SSE stream
2. Map agent lifecycle events to herdr protocol messages
3. Emit `ClientShellSnapshot` on initial connection
4. Emit `PaneSurfacePatch` for incremental updates
5. Handle client input routing back to agents

### Component: crabjar-gpui Client
Location: `crates/crabjar-gpui/src/main.rs`

Features:
- Split-pane layout showing multiple agents simultaneously
- Color-coded agent status indicators (reusing herdr's color system)
- Click-to-focus individual agent panes
- Real-time output streaming per agent
- Scrollback support for agent logs

## Implementation Phases

### Phase 1: Protocol Study & Mock Dashboard (Week 1)
- [x] Analyze herdr-protocol crate wire format
- [ ] Build mock data adapter simulating crabjar agents
- [ ] Create basic GPUI window with split panes
- [ ] Validate rendering performance with synthetic load

### Phase 2: Live Integration (Weeks 2-3)
- [ ] Implement CrabjarSurfaceAdapter
- [ ] Connect to real crabjar orchestrator SSE stream
- [ ] Map actual agent events to surface patches
- [ ] Test with multiple concurrent crabjar sessions

### Phase 3: Bidirectional Communication (Weeks 4+)
- [ ] Enable user input routing from GPUI client to agents
- [ ] Implement click-to-focus and pane selection
- [ ] Add command palette for crabjar operations
- [ ] Optimize performance and polish UX

## Key Technical Decisions

1. **Reuse herdr protocol vs build custom**: Reuse herdr's binary protocol for efficiency and proven architecture. The protocol is already designed for exactly this use case (multiple concurrent terminal surfaces with delta updates).

2. **GPUI dependency scope**: Make GPUI optional via Cargo feature flag (`--features gpu-ui`). Keep existing ratatui TUI as default for environments without GPU or where herdr-gpui isn't available.

3. **Agent-to-pane mapping**: Each crabjar agent gets its own pane in a grid layout. Layout adapts dynamically as agents are spawned/completed.

4. **State synchronization**: Use revision numbers from herdr protocol (`projection_revision`, `surface_revision`) to ensure consistent state between orchestrator and client.

## Performance Considerations

- Binary serialization reduces bandwidth vs JSON/SSE for high-frequency updates
- Delta patches minimize rendering work (only changed cells redrawn)
- GPU acceleration handles many concurrent panes efficiently
- Expect 60fps with <50 concurrent agent views on modern hardware

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| GPUI build complexity | Optional feature flag, detailed build docs |
| Protocol version drift | Pin herdr-gpui dependency version |
| Performance at scale | Benchmark with synthetic load tests |
| Debugging difficulty | Add verbose logging mode for protocol messages |
