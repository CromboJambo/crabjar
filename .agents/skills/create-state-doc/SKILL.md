---
name: create-state-doc
description: |
  Create state-docs Markdown files from directory listings, reference materials, or project overview data. Use whenever the user asks to create a state doc, write a state-doc, generate project state, document a directory, or provides a directory path with documentation intent — even if they don't explicitly say "state doc". Also trigger when the user mentions project documentation, architecture review, or structural overview. Trigger when the user says "create state doc", "write state-doc", "generate state", "document this directory", "review this project", or shares a directory path with documentation intent.
---

# Create State Doc

Creates durable Markdown state-docs for projects, directories, or reference materials. Follows the established format from existing state-docs in `state-docs/`.

## Workflow

### 1. Gather source data

Read the directory, files, or reference materials the user wants documented:

```
Read tool for the directory path
Read tool for key files (Cargo.toml, README.md, AGENTS.md, etc.) if available
```

### 2. Determine project characteristics

From the source data, identify:
- project name and kind (what it is)
- architecture (workspace layout, crates/components)
- core pipeline or primary function
- CLI command surface (if applicable)
- build/test commands
- code quality rules or style guidelines
- security defaults or constraints
- support channel or issues status
- version or current state

### 3. Write the state-doc

Use the established format from existing state-docs (claw-code-state.md, gstack-state.md, opencode-state.md, lms-state.md, zed-acp-orchestrance-state.md):

```markdown
# <project>-state.md

> Generated: <date>
> Source: <source materials>
> Purpose: Human-level review for stateful memory approximation → SQLite indexing

---

## 1. Overview

Brief description of what the project is, its core value proposition, version/current state.

---

## 2. Architecture

### 2.1 Workspace Layout
Directory tree showing structure.

### 2.2 Core Components
Table of key components with role and status.

### 2.3 Core Pipeline or Primary Function
Describe the main workflow or architecture.

### 2.4 <relevant subsection>
Storage, lockfile, CLI commands, etc. as applicable.

---

## 3. Build & Test

Commands for build, test, lint, benchmarks.

---

## 4. Code Quality & Style

Rules, guidelines, style patterns if applicable.

---

## 5. <additional sections as needed>

Next-gen proposals, security, config, etc.

---

## 6. Crabjar Context

### 6.1 Architecture Alignment
Table mapping components to Crabjar's role (Pure observer, append-only, gated, etc.).

### 6.2 State Docs Surface
Crabjar's state-docs commands and overlay system.

### 6.3 Knowledge Bridge
Knowledge bridge description.

### 6.4 Project Config
`.${PROJECT}_config.toml` resolution.

### 6.5 Integration Points
Patterns from this project that ${PROJECT} could adopt.

---

## 7. Confidence Assessment

### 7.1 What This Review Captures
List what the review captures from the source data.

### 7.2 What This Review Might Have Missed
List what might have been missed (unaccessible data, assumptions gaps).

### 7.3 Assumptions
List assumptions made from the source data.

### 7.4 Blind Spots
List blind spots (no access to X, no verification of Y).

### 7.5 Stale After
List conditions that would make this review stale.

---

## 9. Key Takeaways

Numbered takeaways summarizing the review.

---

*End of review.*
```

### 4. Write the file

Write to `state-docs/<project>-state.md`:

```
Write tool for state-docs/<project>-state.md
```

## Guidelines

- **Imperative form**: "Read the directory before writing", not "You should read..."
- **Explain why**: Models respond better to reasons than mandates.
- **Follow existing format**: Use the established structure from existing state-docs as the template.
- **Include doubt**: Every derived output must include doubt block (what might have missed, assumptions, blind spots, stale after).
- **Crabjar context**: Always include Crabjar alignment section mapping this project's components to Crabjar's architecture.
- **Integration points**: Identify patterns from this project that ${PROJECT} could adopt.
- **No comments**: Do not add comments to the state-doc.
- **Imperative takeaways**: Key takeaways should read as plain statements.

## When to skip

- If the directory is empty or has no meaningful content, skip creating a state-doc.
- If the user already has a state-doc for this project, update it instead of creating a new one.
- If the source data is too sparse (<5 files, no README, no config), produce a minimal overview instead of a full review.

## Reference Material

- `references/state-doc-format.md` — full state doc format template and section requirements

## Bundled Scripts

- `scripts/state_doc_generate.sh` — generate state doc from directory path
