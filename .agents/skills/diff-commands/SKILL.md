---
name: diff-commands
description: |
  Implement diff commands for page state comparison. Use whenever the user wants to compare page states with baselines — diff snapshot, diff screenshot, diff url for detecting page changes. Trigger when the user mentions diff commands, page comparison, page change detection, state diff, or wants to build page change detection tooling.
---

# Diff Commands

## Core Concept

Diff commands compare page states with baselines for detecting changes. Three diff modes: snapshot (accessibility tree), screenshot (visual), url (navigation).

## Diff Types

| Type | Comparison | Output |
|---|---|---|
| `diff snapshot` | Accessibility tree vs baseline | Element changes |
| `diff screenshot` | Visual vs baseline | Visual changes |
| `diff url` | Current URL vs baseline | Navigation changes |

## Implementation

### 1. Snapshot Diff

Compare accessibility trees:
- **Baseline**: snapshot at time T0
- **Current**: snapshot at time T1
- **Diff**: element additions, removals, changes
- **Output**: diff report with element refs

### 2. Screenshot Diff

Compare visual states:
- **Baseline**: screenshot at time T0
- **Current**: screenshot at time T1
- **Diff**: visual changes, annotations
- **Output**: diff report with annotated elements

### 3. URL Diff

Compare navigation states:
- **Baseline**: URL at time T0
- **Current**: URL at time T1
- **Diff**: navigation changes, domain changes
- **Output**: diff report with domain info

### 4. Diff Output

Generate diff output:
- **JSON format**: structured diff report
- **Element refs**: diff elements mapped to refs
- **Change types**: addition, removal, modification
- **Confidence**: confidence in diff accuracy

## Configuration

- **Diff mode**: snapshot, screenshot, url
- **Baseline source**: previous snapshot, stored baseline, current state
- **Output format**: JSON, structured, annotated
- **Threshold**: minimum change threshold for detection
- **Ref alignment**: mandatory, optional

## Integration with Crabjar

Crabjar's page change detection should adopt:
- Snapshot diff for accessibility tree comparison
- Screenshot diff for visual comparison
- URL diff for navigation comparison
- JSON output for structured results
- Element ref alignment for change mapping
- Confidence surface for diff accuracy

## References

Read `state-docs/agent-browser-state.md` section 2.4 for CLI command surface and section 7.5 for stale detection thresholds.
