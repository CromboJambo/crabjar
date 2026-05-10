---
name: element-refs
description: |
  Implement deterministic element refs for stable web element selection. Use whenever the user wants to assign stable identifiers to page elements that survive DOM changes — @eN refs provide deterministic element selection across page reloads and layout changes. Trigger when the user mentions element refs, @eN refs, stable element selection, deterministic refs, or wants to build stable element targeting tooling.
---

# Element Refs

## Core Concept

Element refs (`@eN`) provide deterministic element selection across page changes. Unlike CSS selectors that break on DOM changes, refs survive layout changes, DOM updates, and page reloads.

## Ref Format

```
@e1, @e2, @e3, @e4, ...
```

## Implementation

### 1. Ref Generation

Generate refs from snapshot:
- **Snapshot source**: accessibility tree capture
- **Sequential**: N assigned in DOM order
- **Immutable**: refs fixed at snapshot time
- **Persistent**: refs survive page changes

### 2. Ref Stability

Ensure ref stability:
- **DOM changes**: refs survive element repositioning
- **Page reload**: refs survive content changes
- **Layout shift**: refs survive CSS changes
- **Dynamic content**: refs survive JS updates

### 3. Ref Matching

Match refs to elements:
- **Snapshot match**: ref ↔ element at snapshot time
- **Current match**: ref ↔ element at current time
- **Stability check**: verify ref still matches element
- **Mismatch handling**: surface uncertainty on mismatch

### 4. Ref Usage

Use refs for actions:
- **click @eN**: click element with ref @eN
- **fill @eN**: fill element with ref @eN
- **get text @eN**: retrieve text from element with ref @eN
- **batch**: multi-step actions with refs

## Configuration

- **Ref format**: @eN, custom format, alternative
- **Ref scope**: full page, specific region, specific roles
- **Stability check**: mandatory, optional, disabled
- **Mismatch handling**: surface uncertainty, ignore, retry
- **Ref lifespan**: snapshot time, session time, persistent

## Integration with Crabjar

Crabjar's stable element selection should adopt:
- @eN refs as deterministic identifiers
- Snapshot-based ref generation
- Stability verification across page changes
- Mismatch uncertainty surface
- Ref usage for action mapping

## References

Read `state-docs/agent-browser-state.md` section 2.4 for CLI command surface and section 7.5 for stale detection thresholds.
