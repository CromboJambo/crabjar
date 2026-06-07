---
name: comparity
description: |
  Compare feature parity between crabjar and another repo. Use whenever the user
  says "compare parity", "parity check", "compare with X", "what does X have that we don't",
  "how do we stack up against X", "learn from X's design", or mentions comparing against
  another project — even without using the word "parity" explicitly. Trigger on any
  comparison intent: "is X orthogonal?", "should we fork X?", "learn from X's decisions",
  "what's the gap between us and X?"
---

# Comparity — Feature Parity Comparison

Compare crabjar against an external project to identify gaps, overlaps, and design influence.

## Workflow

### 1. Identify the target

Get the repo URL or local path from the user. If it's a URL, clone/fetch the README, Cargo.toml (or package.json), and architecture docs. For local projects, read directly.

Key files to fetch:
- `README.md` — project overview, features, status
- `Cargo.toml` or `package.json` — workspace structure, dependencies, version
- `AGENTS.md` or `CLAUDE.md` — agent rules, architecture decisions
- `project_map.md` or equivalent — structural map
- Architecture docs (e.g., `docs/book/src/architecture/`) — deep dive into design decisions
- `ROADMAP.md` or `CHANGELOG.md` — direction and maturity

### 2. Extract crabjar's surface

Read these from the workspace:
- `project_map.md` — section 6 (CLI commands), section 9 (crabjar context)
- `README.md` — architecture, constraints
- `Cargo.toml` — workspace members
- `.crabjar_config.toml` — config surface

### 3. Extract target's surface

Read the equivalent files from the target repo. Focus on:
- CLI commands or API surface
- Core architecture (crates, modules, layers)
- Security model
- Memory/knowledge system
- Execution model
- Plugin/extension system
- Configuration approach

### 4. Build the comparison

Produce three tables:

**Table 1: Feature parity**

| Target Feature | CrabJar Equivalent | Status | Notes |
|---|---|---|---|
| Feature A | `crate/path` | ✅ wired | Same pattern |
| Feature B | — | ❌ Not built | Gap |
| Feature C | `other-crate` | ⚠️ Partial | Different approach |

**Table 2: What target has that crabjar doesn't**

List unique features of the target that represent genuine gaps.

**Table 3: What crabjar has that target doesn't**

List crabjar's unique contributions.

### 5. Analyze design influence

Answer:
- Is the target orthogonal enough to crabjar? (Can they compose?)
- Should we fork it? (Usually no — composition > forking)
- What design decisions from the target are worth learning from?
- What's the right path: adopt, adapt, or ignore?

### 6. Credit upstream

If the target influenced crabjar's design (like ZeroClaw did), add attribution to:
- `README.md` — new "Acknowledgments" section
- `project_map.md` — inline credit in relevant sections + provenance entry

Use the pattern:

```markdown
**Acknowledgments**

[Project] docs inspired:
- [Feature] — [specific influence]
- [Feature] — [specific influence]

[Project]'s contribution: [what they did differently]
Crabjar's contribution: [what we do differently]
```

Add a provenance entry to `project_map.md` table:

| UUID | Item | Set At | Reason | Source |
| `prov-comparity-<target>` | README + project_map parity credit: <target> | `<date>` | Feature parity analysis | crabjar/README.md, crabjar/project_map.md |

## Output format

Return the comparison as structured output:

1. **Summary** (2-3 lines): orthogonality verdict
2. **Feature parity table** (Table 1)
3. **Gap analysis** (Tables 2 + 3)
4. **Design analysis** (orthogonality, fork viability, influence worth)
5. **Recommendation** (compose, adopt, fork, or ignore)
