---
name: annotated-screenshots
description: |
  Implement annotated screenshots for visual element discovery. Use whenever the user wants to overlay numbered labels on interactive elements for multimodal reasoning — annotated screenshots enable visual element identification matching @eN refs. Trigger when the user mentions annotated screenshots, visual element discovery, multimodal reasoning, numbered labels, or wants to build visual element targeting tooling.
---

# Annotated Screenshots

## Core Concept

Annotated screenshots overlay numbered labels on interactive elements for multimodal reasoning. The numbered labels match `@eN` refs from snapshots, enabling visual element identification that aligns with deterministic refs.

## Implementation

### 1. Element Identification

Identify interactive elements:
- Buttons, inputs, links, checkboxes, selects
- Hoverable elements
- Draggable elements
- Uploadable elements
- Dialog triggers

### 2. Label Assignment

Assign numbered labels to elements:
- **Sequential**: 1, 2, 3, ... based on DOM order
- **Match refs**: label N matches @eN ref from snapshot
- **Position**: label placed near element (top-left, top-right, bottom-left)
- **Visibility**: label visible in screenshot overlay

### 3. Screenshot Generation

Generate annotated screenshot:
- **Capture**: screenshot of page viewport
- **Overlay**: numbered labels on interactive elements
- **Description**: text description of annotated elements
- **Ref alignment**: labels match @eN refs from snapshot

### 4. Multimodal Reasoning

Enable multimodal reasoning:
- **Visual**: screenshot with annotations
- **Text**: element descriptions with refs
- **Cross-reference**: visual label N ↔ ref @eN
- **Action**: click @eN ↔ click visual label N

## Configuration

- **Label position**: top-left, top-right, bottom-left, bottom-right
- **Label style**: numbered, colored, sized
- **Element filter**: interactive only, all elements, specific roles
- **Ref alignment**: mandatory, optional, disabled
- **Viewport**: full page, specific region, scrollable

## Integration with Crabjar

Crabjar's visual element discovery should adopt:
- Annotated screenshots as visual element identification
- Label alignment with @eN refs
- Multimodal reasoning support
- Interactive element filtering
- Ref cross-reference for action mapping

## References

Read `state-docs/agent-browser-state.md` section 2.4 for CLI command surface and section 7.5 for stale detection thresholds.
