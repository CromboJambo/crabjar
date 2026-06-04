---
name: knowledge-triage
description: |
  Classify incoming observations into triage categories (auto-insert, suggest, skip,
  quarantine) before writing to the knowledge store. Use whenever the agent detects an
  insight, pattern, error, or structural finding that might belong in the knowledge store.
  Trigger when the agent wants to record a lesson, fix a bug, discover a convention, or
  encounters a recurring problem. Also trigger during dreaming-mode reflection, post-task
  summary, error analysis, or any situation where "this should be remembered" applies.
---

# Knowledge Triage

Decide whether an observation belongs in the knowledge store, and if so, how to write it.
Never insert blindly — every write must pass through this triage.

## Triage Categories

| Category | Criteria | Action |
|----------|----------|--------|
| **Auto-insert** | Recurring pattern (seen 2+ times), structural rule, hard-won lesson | Write directly with `crabjar knowledge insert` |
| **Suggest** | Novel insight, one-off finding, architectural observation | Present to user with `## Knowledge Suggestion` block |
| **Skip** | Obvious facts, transient state, documentation that already exists elsewhere | No action |
| **Quarantine** | Hypothesis, unverified claim, "might be true" statement | Insert as quarantined pending review |

## Decision Workflow

### Step 1: Check for duplication

Before any write, query existing entries:

```bash
sqlite3 <project>/knowledge.db "SELECT id, content, tags, weight FROM knowledge_entries WHERE content LIKE '%<keywords>%';"
```

Also check `knowledge_entries` for tag-based matches:

```bash
sqlite3 <project>/knowledge.db "SELECT id, content, tags FROM knowledge_entries WHERE tags LIKE '%<tag>%';"
```

If a close match exists (same content, overlapping tags, weight > 0.70):
- **Skip** — already stored
- Optionally update weight via deactivate if the entry is stale

### Step 2: Classify the observation

Ask:

1. **Has this been seen before?** (from Step 1) → Auto-insert
2. **Is it a structural rule / convention / hard-won lesson?** → Auto-insert
3. **Is it a novel insight or architectural finding?** → Suggest
4. **Is it unverified or speculative?** → Quarantine
5. **Is it obvious / already documented?** → Skip

### Step 3: Score confidence

Base confidence by category:

| Category | Base confidence |
|----------|----------------|
| Auto-insert | 0.85 — 0.95 |
| Suggest | 0.50 — 0.75 |
| Quarantine | 0.30 — 0.50 |
| Skip | N/A |

Adjust confidence:

- **+0.10** if it prevented a future bug
- **+0.10** if it was discovered through investigation (not just reading)
- **-0.10** if it contradicts an existing entry
- **-0.10** if it's based on a single data point
- **-0.15** if it's a guess or "I think" statement

### Step 4: Execute

#### Auto-insert (confidence >= 0.75):

```bash
crabjar knowledge insert --content "<concise statement>" --kind <context|instruction|pattern|example> --tags <tag1,tag2,tag3>
```

Tags should include:
- At least one structural tag (e.g., `crabjar`, `dotfiles`, `llm-runner`)
- At least one semantic tag (e.g., `symlink`, `workspace`, `dependency`)
- A recurrence tag if applicable: `recurring`

#### Suggest (confidence 0.50 — 0.74):

Present to the user:

```markdown
## Knowledge Suggestion

**Content**: <statement>
**Confidence**: 0.XX
**Tags**: tag1, tag2
**Rationale**: Why this might be worth storing

[ ] Store this in knowledge store? (yes/no/skip)
```

Do not write until the user confirms.

#### Quarantine (confidence < 0.50 or unverified):

```bash
crabjar knowledge insert --content "<statement>" --kind context --tags <tags>
```

Add metadata flagging it as quarantined. The guard gate will route it to pending status.

#### Skip:

No action. If you're tempted to skip something that feels important, re-evaluate it as a "suggest" candidate.

## Integration Points

Activate this skill at these moments in the agent workflow:

1. **After fixing a bug** — the fix pattern is worth recording
2. **After a structural change** (move, rename, restructure) — the lesson is worth recording
3. **After a failed command** — the cause and resolution are worth recording
4. **During dreaming-mode reflection** — synthesized patterns get triaged
5. **When the user shares context** — structural info the agent needs to remember
6. **When discovering a convention** — project-specific pattern worth encoding
7. **Before writing to a config file** — the convention being followed is worth recording

## When NOT to Activate

- The observation is a one-time operational detail (e.g., "ran `just check`")
- The information is already in state-docs, AGENTS.md, or project_map.md
- The finding is purely environmental (e.g., "disk is at 33%")
- The user explicitly says "don't record this"

## Output Contract

Every triage decision produces one of:

1. **Auto-insert**: The `crabjar knowledge insert` command that was executed
2. **Suggest**: The markdown suggestion block above
3. **Skip**: No output (or a brief "skipped: <reason>" if the user asks)

## Boundary with Other Skills

- **`dreaming-mode`**: Produces synthesized patterns. `knowledge-triage` triages those patterns for storage.
- **`session-backtrace`**: Recovers lost context. `knowledge-triage` records new knowledge.
- **`attention-logger`**: Manages short-term attention. `knowledge-triage` manages long-term durable knowledge.
- **`dotfiles-breach`**: Handles secrets exposure. `knowledge-triage` records what was found if it's a structural lesson.

## Key Principle

The knowledge store is for **structural knowledge** — things that would be worth knowing if you came back to this project after a month. Not for operational logs, not for obvious facts, not for transient state. If you're unsure, suggest it rather than inserting it.
