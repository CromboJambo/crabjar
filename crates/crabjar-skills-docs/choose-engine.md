---
name: crabjar-choose-engine
description: Use when starting a coding task and need to decide between Hermes (built-in), Claude Code (complex multi-file), or Codex (targeted single-file)
version: 1.0.0
tags: [routing, orchestration, engine, crabjar, decision]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [guard]
---

## Overview

Engine routing decision. Chooses the right coding engine based on task complexity, file count, and modification scope.

## When to Use

- Starting a coding task and unsure which engine to use
- The user asks "which tool should I use?" or "how should I implement this?"
- Before any implementation that modifies code

## Decision Matrix

| Factor | Hermes | Claude Code | Codex |
|---|---|---|---|
| File count | 1-2 files | 3+ files | 1 file |
| Complexity | Simple, well-defined | Complex, needs reasoning | Targeted fix |
| Scope | Local changes | Cross-module | Single function/file |
| Context needed | Minimal | Large context window | Minimal |

## Procedure

**1. Assess the task:**

```
- How many files will be modified?
- Is the change localized or cross-cutting?
- Does it require architectural reasoning or just pattern matching?
- What is the user's preference (if any)?
```

**2. Route:**

- **Hermes** (built-in): 1-2 files, simple changes, no external tool needed
- **Claude Code**: 3+ files, complex refactoring, architectural decisions, needs large context
- **Codex**: Single file fix, targeted change, quick edit

**3. Record the decision:**

```bash
crabjar guard record \
  --type engine-choice \
  --result "[engine-name]" \
  --detail '{"files_affected": N, "complexity": "low|medium|high", "reason": "..."}'
```

**4. Execute:**

- If Hermes: proceed with built-in file editing
- If Claude Code: invoke via `hermes delegate` with Claude Code skill
- If Codex: invoke via `hermes delegate` with Codex skill

## Pitfalls

- Over-engineering: don't route to Claude Code for a simple typo fix.
- Under-engineering: don't use Hermes for a 10-file refactoring — it will miss context.
- Always record the engine choice in guard for audit trail.
- If the user has a preference, honor it unless the task clearly needs a different engine.

## Verification

- Guard record shows the engine choice with reasoning
- The chosen engine successfully completes the task
- No unnecessary tool invocation (e.g., Claude Code for a one-line fix)
