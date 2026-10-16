---
name: crabjar-create-skill
description: Use when adding a new capability to the agent that does not exist in the current skill pack — crabjar-backed skill creation and indexing
version: 1.0.0
tags: [meta, skills, creation, crabjar, skill-reference-store]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [skill-reference-store]
---

## Overview

Meta-skill. Guides creation of a new SKILL.md file, installs it to `~/.hermes/skills/`, and indexes it into crabjar's skill-reference-store for staleness tracking.

## When to Use

- A workflow keeps recurring that has no matching skill
- User asks "can the agent learn to do X automatically?"
- Extending the agent with project-specific skills

## Prerequisites

- Clear idea of what the skill should do
- crabjar installed (for skill-reference-store indexing)

## Procedure

Collect from user before generating:

1. **Skill name** — lowercase, hyphenated (`create-github-pr`)
2. **Trigger description** — "Use when..." (this is what the agent uses for matching)
3. **When to use** — specific conditions that fire the skill
4. **Procedure** — step-by-step, concrete and testable
5. **Prerequisites** — tools, credentials, prior state
6. **Known pitfalls** — only real failure modes

Generate the skill file:

```markdown
---
name: [skill-name]
description: Use when [specific triggering conditions — not workflow summary]
version: 1.0.0
tags: [tag1, tag2]
---

## Overview
[1-2 sentences: what this skill is]

## When to Use
[bullet list of specific triggers]

## Prerequisites
[everything required before running]

## Procedure
[numbered, concrete, testable steps]

## Pitfalls
[only observed failure modes]

## Verification
[concrete check that skill completed successfully]
```

Write to:
1. `~/.hermes/skills/[skill-name].md` — immediate load by the agent
2. `[project]/crabjar-skills/[skill-name].md` — repo persistence (if accessible)

Index into crabjar's skill-reference-store:
```bash
crabjar skill index ~/.hermes/skills/[skill-name].md
```

Save to memory: key `skill-created-[skill-name]`, value `{ name, description, date }`.

Offer to test the new skill immediately.

## Pitfalls

- The `description` field drives agent matching. It MUST start "Use when..." and describe ONLY triggering conditions — never the skill's workflow.
- Procedure steps must be concrete and testable. Remove aspirational steps.
- Pitfalls section: only failures that have actually occurred, not hypotheticals.
- Always confirm the file was written to `~/.hermes/skills/` — not just the repo.
- After creating a skill, run `crabjar skill verify [skill-name]` to validate the index.

## Verification

- `~/.hermes/skills/[skill-name].md` exists
- `crabjar skill list` shows the skill indexed
- Tell the agent "load skill [skill-name]" — it responds correctly
- Skill saved to memory
