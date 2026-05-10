---
name: semantic-locators
description: |
  Implement semantic element locators for web element discovery. Use whenever the user wants to find elements on a page without CSS selectors — semantic locators provide deterministic element selection by role, text, label, placeholder, alt, title, testid. Trigger when the user mentions element discovery, semantic locators, find role, find text, find label, or wants to build web element targeting tooling.
---

# Semantic Locators

## Core Concept

Semantic locators provide deterministic element selection without CSS selectors. They identify elements by their semantic properties rather than structural position.

## Locator Types

| Locator | Description | Example |
|---|---|---|
| `find role` | Find by accessibility role (button, input, heading, link, etc.) | `find role button` |
| `find text` | Find by visible text content | `find text "Submit"` |
| `find label` | Find by associated label text | `find label "Email"` |
| `find placeholder` | Find by placeholder attribute | `find placeholder "Enter name"` |
| `find alt` | Find by alt text (images) | `find alt "Logo"` |
| `find title` | Find by title attribute | `find title "Help"` |
| `find testid` | Find by test identifier | `find testid "submit-btn"` |
| `find first` | Find first matching element | `find first role button` |
| `find last` | Find last matching element | `find last role link` |
| `find nth` | Find nth matching element | `find nth role input 2` |

## Implementation

### 1. Accessibility Tree Query

Query the page's accessibility tree for elements matching the locator criteria.

### 2. Result Filtering

Filter results by:
- Role type (button, input, heading, link, etc.)
- Text content (exact match, substring, regex)
- Label association (for, aria-labelledby)
- Placeholder attribute
- Alt text attribute
- Title attribute
- Test identifier attribute

### 3. Positional Selection

For multiple matches:
- `first` — return first match in DOM order
- `last` — return last match in DOM order
- `nth` — return nth match (0-indexed)

### 4. Ref Assignment

Assign deterministic `@eN` refs to matched elements for subsequent action.

## Configuration

- **Search scope**: restrict to specific region of page (frame, tab, container)
- **Match mode**: exact, substring, regex
- **Result limit**: max number of matches to return
- **Ref assignment**: assign @eN refs to results

## Integration with Crabjar

Crabjar's web element discovery should adopt:
- Semantic locators as primary element targeting method
- Accessibility tree as source data
- Deterministic refs for stable selection
- Positional selection for multiple matches

## References

Read `state-docs/agent-browser-state.md` section 2.4 for the CLI command surface and section 7.5 for stale detection thresholds.
