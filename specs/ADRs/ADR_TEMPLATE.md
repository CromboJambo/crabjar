---
id: ADR-NNN
title: <title>
status: Proposed | Accepted | Superseded | Deprecated
date: YYYY-MM-DD
supersedes: [[ADR-NNN]] (if applicable)
see_also: [[ADR-NNN]], [[ADR-NNN]] (if applicable)
---

# ADR-NNN: <title>

## Status

What is the status?

- **Proposed**: The team's current best understanding of this decision. Not yet reviewed.
- **Accepted**: The team has discussed this, and it represents a course change.
- **Superseded**: This ADR describes a past state. Another ADR has superseded this decision.
- **Deprecated**: No longer relevant or desirable.

## Context

> "A decision log is a journal of the author regarding all kinds of decisions that were made in the course of a project to build software." — [Michael Nygard, "Documenting Architecture Decisions", 2011](https://www.infoq.com/articles/Architecture-Decision-Lang)

Describe the context and problem statement. What issue are we solving? What forces are at play? Include relevant constraints (technical, organizational, temporal).

This section should enable anyone to understand *why* this decision needed to be made. If it's obvious when you read the decision, you haven't written enough context.

## Decision

What is the change that we're proposing or have decided to do?

Be specific and concise. This is the core of the ADR — one clear statement of what was chosen.

## Options Considered

> "Every architecture decision has alternatives." — Nygard, 2011

Describe the relevant options and trade-offs considered. For each option, briefly note:
- What it offers
- Why it was accepted or rejected

You don't need exhaustive analysis — just enough to show the decision wasn't arbitrary. Common patterns:
- **Do nothing / status quo** — what happens if we don't act?
- **Alternative A** — why considered, why not chosen (or vice versa)
- **Alternative B** — same treatment

## Consequences

> "Every action has consequences." — Nygard, 2011

What becomes easier or more difficult now that this decision has been made?

### Positive consequences
- What benefits does this enable?

### Negative consequences (trade-offs)
- What costs do we accept?
- What capabilities are foregone?
- What new risks emerge?

### Ongoing concerns
- What needs monitoring or re-evaluation?
- When should this decision be revisited?

## References

- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/README.md` — ADR process and conventions for this project
