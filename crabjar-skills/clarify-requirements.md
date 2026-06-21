---
name: crabjar-clarify-requirements
description: Use when a new project or significant feature starts and requirements need to be gathered, structured, and saved to the knowledge store
version: 1.0.0
tags: [requirements, planning, memory, crabjar, knowledge-store]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [knowledge-store]
---

## Overview

Structured requirements gathering. Asks 7 focused questions, saves answers to crabjar's knowledge store, and creates a structured requirements artifact for downstream use.

## When to Use

- Starting a new project or significant feature
- The user says "I want to build..." or "Let's start a new app"
- Requirements are vague and need structuring before implementation

## Prerequisites

- crabjar installed
- A clear project name or topic

## Procedure

**1. Ask the 7 structured questions:**

1. **What problem does this solve?** — Who has it and how bad is it?
2. **Who is the target user?** — Persona, role, context
3. **What are the top 3 must-have features?** — Ranked by priority
4. **What should NOT be built?** — Explicit out-of-scope items
5. **What tech stack preferences exist?** — Or "no preference, recommend"
6. **What is the timeline?** — Deadline, milestones, or "no rush"
7. **What are the success criteria?** — How do we know it worked?

**2. Save to knowledge store:**

```bash
crabjar knowledge store \
  --key "requirements-[project-name]" \
  --type structured \
  --data '{
    "problem": "...",
    "user": "...",
    "features": [{"name": "...", "priority": 1}, ...],
    "out_of_scope": ["..."],
    "tech_stack": "...",
    "timeline": "...",
    "success_criteria": ["..."]
  }'
```

**3. Save to memory:**

Save to memory: key=`requirements-[project-name]`, value=`{ date, feature_count, timeline }`.

**4. Confirm with user:**

Present the structured requirements back to the user for confirmation. Ask: "Does this capture what you need?"

**5. Next step:**

If requirements are clear → suggest `crabjar-product-brief` to generate the brief.
If requirements are unclear → ask follow-up questions before proceeding.

## Pitfalls

- Do not skip this step. Vague requirements lead to wasted implementation work.
- Ask all 7 questions — even if the user thinks they've answered them. Structured format catches gaps.
- "No preference" on tech stack is a valid answer — recommend based on project type.
- Success criteria must be measurable — "it works" is not a success criterion.

## Verification

- `crabjar knowledge get --key "requirements-[project-name]"` returns structured data
- Memory key `requirements-[project-name]` exists
- User has confirmed the requirements are accurate
