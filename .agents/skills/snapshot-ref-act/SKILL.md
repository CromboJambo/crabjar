---
name: snapshot-ref-act
description: |
  Implement the snapshot-ref-act loop for web content interaction. Use whenever the user wants to interact with web pages via AI agents — snapshot → deterministic element refs → click/fill/get text workflow. Trigger when the user mentions web interaction, element targeting, page snapshot, @eN refs, or wants to build webfetch tooling that adopts this pattern from agent-browser.
---

# Snapshot-Ref-Act Loop

## Core Workflow

The snapshot-ref-act loop is the optimal AI agent workflow for web page interaction:

1. **Snapshot**: capture the accessibility tree of a page
2. **Refs**: generate deterministic element references (`@eN`) from the snapshot
3. **Act**: use those refs to click, fill, get text, or other actions

## Implementation

### 1. Snapshot Layer

Capture the page's accessibility tree. This provides:
- Semantic element identification (role, text, label, placeholder, alt, title, testid)
- Deterministic refs that survive page layout changes
- Content boundary markers for LLM safety

### 2. Ref Generation

From the snapshot, assign each element a deterministic ref `@eN` where N is a sequential index. These refs:
- Survive DOM changes (unlike CSS selectors)
- Match annotated screenshots (numbered labels on interactive elements)
- Enable stable element selection across page reloads

### 3. Action Layer

Use refs to execute actions:
- `click @e1` — click element with ref @e1
- `fill @e2 "value"` — fill element with ref @e2
- `get text @e3` — retrieve text from element with ref @e3
- `batch` — multi-step workflow without per-command process startup overhead

## Configuration

- **Content boundaries**: wrap page output in delimiters for LLM safety
- **Domain allowlist**: restrict navigation to trusted domains
- **Action policy**: gate destructive actions via JSON file
- **Element refs**: `@eN` deterministic refs from snapshots

## Integration with Crabjar

Crabjar's webfetch tooling should adopt this pattern:
- Snapshot → refs → click/fill/get text as the core workflow
- Semantic locators for deterministic element selection without CSS selectors
- Content boundary markers for webfetch output
- Domain allowlist for webfetch scope gating
- Action policy for authorization layer
- Annotated screenshots for visual element discovery
- Batch execution for multi-step workflow efficiency
- Element refs for stable element selection across page changes
- Diff commands for page change detection

## References

Read `state-docs/agent-browser-state.md` section 6.4 for the full integration points list and section 2.4 for the CLI command surface.
