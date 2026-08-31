# Architecture Decision Records

This directory contains the project's Architecture Decision Records (ADRs). An ADR captures a significant architectural decision — the context, rationale, and consequences of a choice that has lasting impact on the system.

## Format

All ADRs follow the [Nygard template](ADR_TEMPLATE.md), adapted from Michael Nygard's ["Documenting Architecture Decisions"](https://www.infoq.com/articles/Architecture-Decision-Lang) (2011). Each file uses:

```
specs/ADR-NNN_<title>.md
```

Where `NNN` is a zero-padded sequential number. The title should be a noun phrase describing the decision, using kebab-case for word separation.

## Status Values

| Status | Meaning |
|--------|---------|
| **Proposed** | Under discussion; not yet accepted by the team |
| **Accepted** | Discussed and adopted as course change |
| **Superseded** | Replaced by a later ADR (see `supersedes` field) |
| **Deprecated** | No longer relevant or desirable |

## Cross-Referencing

ADRs reference each other using the same inline link pattern used throughout this project:

```markdown
[[ADR-NNN]]
```

Examples:
- `See also: [[ADR-002]]` — related decision, no ordering implied
- `Supersedes: [[ADR-001]]` — replaces an earlier decision
- `Replaced by: [[ADR-005]]` — this ADR is now superseded

This matches the reference style used in `.agents/skills/git-reflector/` for linking to DecisionBlobs and mirror-log entries.

## When to Write an ADR

Write one when a decision is:
- **Architecturally significant** — affects multiple components or modules
- **Hard to reverse** — would require substantial rework to undo
- **Non-obvious** — the rationale isn't self-evident from code alone
- **A precedent** — establishes a pattern for future decisions

Don't write one for:
- Trivial implementation details (pick a variable name, choose an internal helper)
- Obvious choices (use `thiserror` because it's already in every crate)
- Temporary hacks with known expiration dates (note the deadline inline instead)

## When NOT to Write an ADR

Skip the formal process when:
- The decision is reversible and low-cost
- It's a local detail that doesn't affect other teams or components
- You'd spend more writing the ADR than making the decision

## Creating a New ADR

1. Copy `ADR_TEMPLATE.md` to `specs/ADR-NNN_<title>.md` (next available number)
2. Fill in all sections — especially **Context** and **Options Considered**
3. Set status to `Proposed` initially
4. Discuss with the team; update to `Accepted` when agreed
5. Update this README's index table
6. If it supersedes an earlier ADR, add `supersedes: [[ADR-NNN]]` and mark the old one as `Superseded`

## Index

| # | Title | Status | Date |
|---|-------|--------|------|
| 001 | Specify Decision Process | Accepted | 2026-07-08 |
| 002 | Herdr as Execution Substrate | Proposed | 2026-08-23 |
| 003 | Spatial Habitat Layer | Proposed | 2026-08-23 |
| 004 | The Glass — Abstraction Boundary Orientation | Proposed | 2026-08-30 |
| 005 | Typed Terminal Stream as Session Substrate | Proposed | 2026-08-30 |
| 006 | Attempt Graph as Falsification Record | Accepted | 2026-08-30 |
