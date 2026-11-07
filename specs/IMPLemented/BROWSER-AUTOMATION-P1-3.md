# Browser Automation (P1-3 Drop-in Replacement Blocker)

**Status:** In Progress  
**Priority:** P1-3 (Drop-in replacement blocker)  
**Target:** crabjar-browser crate  
**Estimated effort:** 5-7 days  

## Overview

Provide browser automation capabilities that match Hermes' `browser_exec(code)` tool. This enables agents to automate login flows, form submissions, dynamic page interactions, and other interactive web tasks that cannot be accomplished with simple HTTP requests.

## Architecture Decision

After evaluation, the recommended approach is **subprocess integration with Playwright** via Python wrapper script:

### Why Playwright?
- Cross-platform (Linux, macOS, Windows)
- Supports all major browsers (Chromium, Firefox, WebKit)
- Excellent JavaScript execution support
- Active development and large community
- Can be installed via pip for easy deployment

### Alternative Approaches Evaluated
1. **Puppeteer via Node subprocess** — Good but requires Node.js runtime; Playwright has better cross-browser support
2. **Direct Browser Use CLI integration** — Hermes' approach, but tightly coupled to their specific tooling
3. **Custom implementation with webkit2gtk** — Too complex for initial implementation; would require significant development time

### Integration Strategy
- Create `crabjar-browser` crate that wraps Playwright Python script execution
- Provide safe subset of browser automation operations through structured API
- Execute Playwright scripts via subprocess with JSON input/output protocol
- Integration with crabjar's execution pipeline (guard → concierge → telemetry)

## Public API Design

```rust
// Simple navigation and content extraction
let result = browser::navigate("https://example.com").await?;
println!("{}", result.title);
println!("{}", result.content);

// JavaScript execution
let value = browser::evaluate("https://example.com", "document.title").await?;

// Form filling and submission
browser::fill_form("https://example.com/login", &[
    ("username", "user@example.com"),
    ("password", "secret"),
]).await?;

// Screenshot capture
let screenshot = browser::screenshot("https://example.com").await?;
```

## Security Considerations

- Browser sandboxing via Playwright's isolated context feature
- Cookie isolation per session to prevent credential leakage
- Credential management through crabjar's secrets bridge (future)
- Timeout management to prevent hung browser instances

## Implementation Plan

1. Create `crabjar-browser` crate with basic structure
2. Implement subprocess wrapper for Playwright script execution
3. Define JSON input/output protocol for browser operations
4. Implement core operations: navigate, evaluate, screenshot
5. Add form filling and submission support
6. Integration tests with local test server
7. Update CHANGELOG.md and spec file

## Testing Strategy

- Unit tests for subprocess wrapper and JSON protocol
- Integration tests using local HTTP test server (similar to P1-2 approach)
- End-to-end tests against real websites (with proper error handling)