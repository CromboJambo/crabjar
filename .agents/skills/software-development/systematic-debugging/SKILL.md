---
name: systematic-debugging
description: |
  Use when performing 4-phase root cause debugging: understand bugs before fixing, reproduce consistently, isolate root cause, and verify fix. Trigger this when the user says "debug this" or needs structured approach to bug investigation.
category: software-development
tags: [debugging, troubleshooting, root-cause]
---

# Systematic Debugging

This skill provides a structured 4-phase approach to debugging that emphasizes understanding bugs before fixing them, ensuring consistent reproduction and thorough root cause analysis.

## Core Workflow

1. **Phase 1 - Understand**: Gather all information about the bug (symptoms, frequency, environment)
2. **Phase 2 - Reproduce**: Establish consistent reproduction steps
3. **Phase 3 - Isolate**: Narrow down to root cause through systematic elimination
4. **Phase 4 - Verify Fix**: Implement fix and verify it resolves issue without regressions

## Usage Instructions

### Input Requirements
* Bug description or symptoms
* Environment details (OS, version, dependencies)
* Reproduction steps (if known)

### Example Prompts
* `/systematic-debugging investigate this intermittent crash`
* `Apply 4-phase debugging approach to this bug report`
* "Help me systematically debug this issue"

## Reference Files
* `references/debugging-checklist.md`: Comprehensive debugging checklist by phase
* `references/common-root-causes.md`: Catalog of common root causes and patterns

## Bundled Scripts
None — this skill provides debugging methodology and guidance.