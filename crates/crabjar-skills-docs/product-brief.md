---
name: crabjar-product-brief
description: Use when requirements are saved to memory and a written product brief artifact needs to be generated from them
version: 1.0.0
tags: [planning, brief, requirements, crabjar, knowledge-store]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [knowledge-store]
---

## Overview

Generates a structured product brief from captured requirements. Saves the brief to the workspace and indexes it in crabjar's knowledge store for later retrieval.

## When to Use

- Requirements have been gathered and saved to memory
- A written artifact is needed before implementation starts
- The user asks for a "product brief" or "project spec"

## Prerequisites

- Requirements saved to memory (from `clarify-requirements` skill)
- crabjar installed

## Procedure

**1. Load requirements from memory:**

```bash
# Read requirements from the knowledge store
crabjar knowledge get --key "requirements-[project-name]"
```

**2. Generate the brief:**

Structure:
```markdown
# Product Brief: [Project Name]

## Problem
[What problem does this solve? Who has it?]

## Solution
[What are we building? High-level description.]

## Scope
### In Scope
- [Feature 1]
- [Feature 2]

### Out of Scope
- [Not building this]

## Tech Stack
- [Frontend]: [technology]
- [Backend]: [technology]
- [Database]: [technology]
- [Hosting]: [technology]

## Key Decisions
1. [Decision 1] — rationale
2. [Decision 2] — rationale

## Milestones
1. [Milestone 1] — [target date]
2. [Milestone 2] — [target date]

## Risks
- [Risk 1] — mitigation
- [Risk 2] — mitigation
```

**3. Save to workspace:**

```bash
echo "[brief content]" > PRODUCT_BRIEF.md
```

**4. Index in knowledge store:**

```bash
crabjar knowledge store \
  --key "product-brief-[project-name]" \
  --file PRODUCT_BRIEF.md \
  --tags "brief,planning,project-name"
```

**5. Save to memory:**

Save to memory: key=`product-brief-[project-name]`, value=`{ date, path, key-features: [...] }`.

## Pitfalls

- The brief should be concise — 1-2 pages max. If it's longer, split into a separate design doc.
- Scope sections must be explicit and unambiguous. "Maybe later" is not a valid scope entry.
- Tech stack decisions should include rationale, not just tool names.
- Always index the brief in the knowledge store — it's the reference for all downstream work.

## Verification

- `PRODUCT_BRIEF.md` exists in workspace root
- `crabjar knowledge get --key "product-brief-[project-name]"` returns the brief
- Memory key `product-brief-[project-name]` exists with correct metadata
