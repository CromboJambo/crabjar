---
id: ADR-001
title: Specify Decision Process
status: Accepted
date: 2026-07-08
see_also: [[ADR_TEMPLATE.md]]
---

# ADR-001: Specify Decision Process

## Status

**Accepted**. This ADR establishes the decision process for the project. It is self-referential — it creates the mechanism by which future decisions will be recorded.

## Context

Crabjar has accumulated architectural knowledge through multiple channels with no unified record:
- **ROADMAP.md** tracks priorities and open questions but lacks rationale for why certain paths were chosen over others
- **agent_config.md** captures operational principles but not specific technical trade-offs
- **AGENTS.md** documents conventions but not the "why" behind them
- **Git history** contains commits with messages like "Decided: ..." but no structured reasoning

This creates three problems:
1. **Repeated debates**: Without a record of past decisions, the same questions get re-lit (e.g., plugin language support was debated multiple times before being settled)
2. **Lost rationale**: When someone encounters an unfamiliar pattern in the codebase, they can't easily find why it exists
3. **Onboarding friction**: New contributors have no way to understand architectural history without reading every commit message

The project already has a culture of lightweight documentation (see `agent_config.md` at 86 lines, AGENTS.md conventions). Any decision process should match that bar — concise, structured, but not bureaucratic.

## Decision

All significant architectural decisions will be recorded as ADRs in the `specs/` directory using the Nygard-style template (`ADR_TEMPLATE.md`). Each ADR captures:
- **Status** (Proposed → Accepted → Superseded/Deprecated)
- **Context** (why this decision needed to be made)
- **Decision** (what was chosen, stated concisely)
- **Options Considered** (alternatives and trade-offs)
- **Consequences** (positive and negative outcomes of the choice)

Cross-references between related ADRs use `[[ADR-NNN]]` inline links.

This process is lightweight: a single Markdown file per decision, no tooling overhead, no database, no approval workflow beyond team discussion. The index lives in `specs/README.md`.

## Options Considered

### Option 1: Inline comments in code
Document decisions as comments next to the relevant code.

- **Pros**: Decisions live next to their implementation; impossible to lose track of what they apply to
- **Cons**: Scattered across dozens of files; no single view of architectural history; easily missed during refactoring

### Option 2: Wiki or external tool (Notion, Confluence)
Use a dedicated knowledge base.

- **Pros**: Searchable, structured, collaborative editing
- **Cons**: External dependency; requires context-switching; decoupled from the codebase version control; adds friction to recording decisions in the moment

### Option 3: Markdown files in-repo (chosen)
ADRs as Markdown files alongside the source code.

- **Pros**: Version-controlled with the code; discoverable via `grep` and `find_path`; no external tooling; matches existing documentation patterns (`agent_config.md`, `AGENTS.md`); supports structured cross-referencing via `[[ADR-NNN]]` links
- **Cons**: Requires discipline to maintain; easy to forget to update the index table

### Option 4: Git commit messages as decisions
Rely on conventional commits with detailed bodies.

- **Pros**: Zero additional files; always up-to-date with code changes
- **Cons**: No structured format for options/rationale; hard to query across commits; no status lifecycle (proposed → accepted → superseded); easily lost in a busy history

## Consequences

### Positive consequences
- **Single source of truth** for architectural rationale — anyone can read `specs/` and understand the project's decision history
- **Reduces repeated debates** — past decisions are discoverable, so the same questions don't get re-lit
- **Onboarding aid** — new contributors can trace the evolution of key design choices
- **Lightweight** — no tooling, no database, no process overhead beyond writing a Markdown file

### Negative consequences (trade-offs)
- **Maintenance burden** — the index in `README.md` must be updated with each new ADR; drift is possible (mitigated by the project's existing document freshness protocol)
- **Discipline-dependent** — if people forget to write ADRs, the directory becomes incomplete and misleading
- **Not all decisions are architectural** — distinguishing "architectural" from "implementation detail" requires judgment

### Ongoing concerns
- When should this process be revisited? If the team grows beyond ~5 contributors and ADR count exceeds 20, consider adding tags or a search index.
- The `project_map.md` drift governance (>7 days stale) could apply to `specs/README.md`'s index table — if no new ADRs are added for >30 days while the codebase is actively changing, that's worth flagging.

## References

- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/ADR_TEMPLATE.md` — template for new ADRs
- `specs/README.md` — process conventions and index
- `agent_config.md` — operational principles (complementary to ADRs; config captures *how* we work, ADRs capture *why* we chose specific architectures)
