# Embedded WASM Browser for crabjar (GPUI + herdr Integration)

**Status:** Draft  
**Author:** crombo  
**Date:** 2026-10-17  
**Spec Type:** Architecture / Feature Spec  

---

## Overview

Embed a real browser engine inside crabjar's GPUI desktop application, enabling dynamic multi-instance web browsing with herdr-style pane layout management. Each browser instance runs in its own isolated session with full agent control over navigation, content inspection, and interaction.

**Key capabilities:**
- Spawn multiple browser instances as needed (one per task/agent)
- Dynamic pane layout: split right, split below, tab layers
- User-defined layout configurations
- Full agent control: navigate, evaluate JS, read DOM, interact with elements
- Session isolation per instance
- Integration with crabjar's existing session management (`crabjar-host-webview`)

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    crabjar Desktop App                      │
│                     (GPUI Framework)                        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ BrowserPane │  │ BrowserPane │  │ BrowserPane │         │
│  │ Instance A  │  │ Instance B  │  │ Instance C  │         │
│  │ (Agent 1)   │  │ (Agent 2)   │  │ (User)      │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│  ┌──────▼────────────────▼────────────────▼──────┐         │
│  │           Browser Instance Manager            │         │
│  │  - Lifecycle management                       │         │
│  │  - Session isolation                          │         │
│  │  - Layout coordination                        │         │
│  └──────┬─────────────────────────────────────────┘         │
│         │                                                   │
│  ┌──────▼─────────────────────────────────────────┐         │
│  │           crabjar-host-webview                  │         │
│  │  - Cookie store, OAuth2, token cache           │         │
│  │  - Partition management                        │         │
│  └─────────────────────────────────────────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  Platform Webview Backend                                    │
│  (webkit2gtk / webkit / WebView2)                            │
└─────────────────────────────────────────────────────────────┘
```

### Crate Structure

```hemlock
crabjar/
├── host/host-webview/          # Existing: session management
│   ├── src/controller.rs       # WebViewController (existing)
│   ├── src/partition.rs        # Partition manager (existing)
│   └── ...
├── host/host-browser/          # NEW: actual browser embedding
│   ├── src/lib.rs
│   ├── src/instance.rs         # BrowserInstance - one webview per instance
│   ├── src/layout.rs           # Pane layout management
│   └── src/herdr_integration.rs# herdr pane command interface
├── apps/crabjar-desktop/       # Existing: GPUI desktop app
│   └── src/components/browser_pane.rs  # NEW: GPUI component
```

### Browser Instance Lifecycle

```hemlock
let manager = BrowserInstanceManager::new(config);

// Spawn new instance for agent task
let instance_a = manager.spawn(BrowserConfig {
    url: "https://example.com",
    session_partition: Some("agent-task-123".to_string()),
    headless: false,
}).await?;

// Agent interacts with instance
instance_a.navigate("https://example.com/page2").await?;
let title = instance_a.evaluate("document.title").await?;
instance_a.click("#submit-button").await?;

// Spawn another instance in a new pane
let instance_b = manager.spawn(BrowserConfig {
    url: "https://docs.example.com",
    session_partition: Some("agent-task-456".to_string()),
    headless: false,
}).await?;

// Layout management (herdr-style)
manager.split_pane_right(instance_a.id).await?;
manager.move_instance_to_pane(instance_b.id, PanePosition::Right).await?;

// Cleanup when task completes
instance_a.destroy().await?;
```

---

## Platform Backend Selection

### Option 1: `webview` crate (Recommended for MVP)

**Pros:**
- Cross-platform (WebKit on macOS/Linux, WebView2 on Windows)
- Minimal dependencies
- Well-maintained, production-ready
- Simple API: create window, navigate, evaluate JS, inject scripts

**Cons:**
- No true WASM isolation
- Limited to platform's webview version
- Harder to test headlessly

**Implementation notes:**
```hemlock
// Example using the `webview` crate
let mut wv = webview::builder()
    .with_url("https://example.com")
    .with_size(800, 600)
    .build()?;

let title: String = wv.eval("document.title")?;
```

### Option 2: `webkit2gtk` bindings (Linux-focused)

**Pros:**
- Full WebKit engine access
- Better Linux integration
- More control over rendering

**Cons:**
- Requires GTK dependencies
- Less portable

### Option 3: WASM-based browser (Long-term vision)

Compile a browser engine to WASM and run in-process via `wasmtime`:
- Compile Chromium/WebKit components with emscripten
- Run in `wasmtime` runtime for true isolation
- More complex, higher overhead, but matches the "WASM browser" concept

**Trade-offs:**
- Pros: True isolation, reproducible builds, no platform dependencies
- Cons: Performance overhead, compilation complexity, less mature

**Recommendation:** Start with Option 1 (`webview` crate) for MVP, evaluate WASM approach later.

---

## herdr Integration Design

### Pane Layout Commands

```hemlock
// Split current pane right
herdr.split_right();

// Split current pane below
herdr.split_below();

// Move browser instance to specific pane
herdr.move_to_pane("right");
herdr.move_to_pane("below");
herdr.move_to_pane(2);  // by index

// Tab layers within a pane
herdr.tab_new();
herdr.tab_switch(1);
herdr.tab_close();

// Layout configuration
herdr.save_layout("agent-workflow");
herdr.load_layout("agent-workflow");
```

### User-Defined Layouts

Layouts are saved as JSON configuration files:

```json
{
  "name": "multi-agent-research",
  "panes": [
    {
      "type": "browser",
      "instance_id": "agent-task-123",
      "url": "https://example.com/research",
      "position": { "x": 0, "y": 0, "width": 400, "height": 600 }
    },
    {
      "type": "browser",
      "instance_id": "agent-task-456",
      "url": "https://docs.example.com",
      "position": { "x": 400, "y": 0, "width": 400, "height": 300 }
    },
    {
      "type": "browser",
      "instance_id": "user-view",
      "url": "https://example.com/dashboard",
      "position": { "x": 400, "y": 300, "width": 400, "height": 300 }
    }
  ]
}
```

### Layout Persistence

- Layouts saved to `~/.crabjar/layouts/<name>.json`
- Auto-save on window close (optional)
- Restore layout on app startup (configurable)
- Versioning for layout evolution

---

## API Surface

### BrowserInstance

```hemlock
pub struct BrowserInstance {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub partition_name: Option<String>,
    pub visible: bool,
}

impl BrowserInstance {
    /// Navigate to URL
    pub async fn navigate(&self, url: &str) -> Result<()>;

    /// Evaluate JavaScript expression
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value>;

    /// Click element by selector
    pub async fn click(&self, selector: &str) -> Result<()>;

    /// Fill input field
    pub async fn fill(&self, selector: &str, value: &str) -> Result<()>;

    /// Wait for element to appear
    pub async fn wait_for_element(&self, selector: &str, timeout_ms: u64) -> Result<bool>;

    /// Get page screenshot
    pub async fn screenshot(&self) -> Result<Vec<u8>>;

    /// Destroy instance
    pub async fn destroy(&self) -> Result<()>;
}
```

### BrowserInstanceManager

```hemlock
pub struct BrowserInstanceManager {
    // ...
}

impl BrowserInstanceManager {
    /// Create new manager
    pub fn new(config: BrowserConfig) -> Self;

    /// Spawn new browser instance
    pub async fn spawn(&self, config: InstanceConfig) -> Result<BrowserInstance>;

    /// Get existing instance by ID
    pub fn get_instance(&self, id: Uuid) -> Option<&BrowserInstance>;

    /// List all active instances
    pub fn list_instances(&self) -> Vec<&BrowserInstance>;

    /// Split pane right for instance
    pub async fn split_pane_right(&self, instance_id: Uuid) -> Result<()>;

    /// Split pane below for instance
    pub async fn split_pane_below(&self, instance_id: Uuid) -> Result<()>;

    /// Move instance to specific pane
    pub async fn move_instance_to_pane(&self, instance_id: Uuid, position: PanePosition) -> Result<()>;

    /// Save layout configuration
    pub async fn save_layout(&self, name: &str) -> Result<()>;

    /// Load and apply layout configuration
    pub async fn load_layout(&self, name: &str) -> Result<()>;
}
```

### GPUI Component Integration

```hemlock
// In crabjar-desktop app
import { BrowserPane } from "@crabjar/browser-panes";

fn App() {
    return (
        <div style="display: grid; grid-template-columns: 1fr 1fr;">
            <BrowserPane instanceId={agent_task_123} />
            <BrowserPane instanceId={user_view} />
        </div>
    );
}
```

---

## Implementation Phases

### Phase 1: MVP - Single Browser Instance (Weeks 1-2)

**Goals:**
- Embed single browser window in crabjar-desktop using `webview` crate
- Basic navigation and JS evaluation
- Integration with existing `crabjar-host-webview` session management

**Deliverables:**
- New `host/host-browser` crate with basic BrowserInstance API
- GPUI component for rendering browser pane
- Tests: navigate, evaluate, click

### Phase 2: Multi-Instance + Layout (Weeks 3-4)

**Goals:**
- Support multiple concurrent browser instances
- herdr-style pane splitting (right/below)
- Basic tab layers within panes

**Deliverables:**
- BrowserInstanceManager with layout coordination
- Pane position API
- Tab management
- Tests: multi-instance, layout operations

### Phase 3: Layout Persistence + User Configuration (Weeks 5-6)

**Goals:**
- Save/restore layout configurations
- User-defined layout templates
- Auto-save and restore on app restart

**Deliverables:**
- Layout serialization/deserialization
- Layout manager API
- User configuration UI (basic)
- Tests: save/load layouts

### Phase 4: Advanced Agent Control (Weeks 7-8)

**Goals:**
- Full DOM inspection APIs
- Element interaction (fill, select, hover)
- Screenshot capabilities
- Wait/timeout mechanisms

**Deliverables:**
- Complete BrowserInstance API surface
- Agent-friendly error handling
- Integration tests with real web pages

---

## Testing Strategy

### Unit Tests
- BrowserInstance lifecycle (create, navigate, destroy)
- Layout operations (split, move, tab)
- Session isolation between instances

### Integration Tests
- Real navigation to test websites
- JS evaluation and DOM interaction
- Screenshot capture

### End-to-End Tests
- Multi-agent workflow with separate browser instances
- Layout save/restore across app restarts
- herdr pane command integration

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `webview` crate platform incompatibilities | High | Test on all target platforms early; fall back to webkit2gtk for Linux |
| Memory overhead with multiple instances | Medium | Implement instance pooling and resource limits |
| Layout complexity with many panes | Medium | Keep layout API simple; document limitations |
| Session isolation bugs | High | Thorough testing of partition management |

---

## Open Questions

1. Should browser instances be headless by default? (User preference)
2. How to handle browser crashes — auto-restart or report to agent?
3. Resource limits per instance (memory, CPU)?
4. Integration with crabjar's existing telemetry system?

---

## References

- Existing `crabjar-host-webview` crate: `host/host-webview/`
- herdr terminal multiplexer documentation
- GPUI framework documentation
- `webview` crate: https://docs.rs/webview/latest/webview/
- WebKitGTK bindings: https://gtk-rs.org/gtk3-rs/stable/latest/docs/webkit2gtk/
