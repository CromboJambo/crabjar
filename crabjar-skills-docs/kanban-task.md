---
name: crabjar-kanban-task
description: Use when any agent starts, updates, or completes a unit of work that should be visible on the kanban board — crabjar-backed persistence
version: 1.0.0
tags: [kanban, task, tracking, crabjar, state-docs]
metadata:
  crabjar:
    requires_commands: [crabjar]
    depends_on: [state-docs]
---

## Overview

Kanban task tracking backed by crabjar's state-docs system. Every agent action that moves work forward must call this skill — the kanban is the source of truth for all agents in the loop.

## When to Use

- A GitHub issue is accepted into the workflow.
- An agent starts work, creates a PR, passes review, deploys, or gets blocked.
- The user needs current task state visible on the live board.

## Prerequisites

- crabjar installed and initialized in the workspace
- A concrete task title and verifiable acceptance criteria
- A target assignee profile such as `pm`, `dev`, `security`, `qa`, or `ops`

## Kanban columns

| Status | Meaning |
|---|---|
| Triage | Raw idea or unclear issue, not ready for implementation |
| Todo | Defined work waiting on dependencies or assignment |
| Ready | Assigned work waiting for the dispatcher to spawn the worker |
| Running | Worker profile is actively handling the task |
| Blocked | Worker needs human input or the retry circuit breaker tripped |
| Done | Completed with summary and handoff metadata |

## Procedure

**Create a new task (PM, after triaging an issue):**

```bash
crabjar kanban create "Fix: [what] — Issue #[n]" \
  --assignee dev \
  --priority [score] \
  --body "Why: [one sentence]. Acceptance: [verifiable checks]."
```

Acceptance criteria must be a concrete, verifiable check — not "it works." If you cannot write a specific check, ask for clarification before creating the card.

Save returned task ID to memory: key `task-id-issue-[n]`.

**Claim and move to running (Dev):**

```bash
crabjar kanban claim [task-id]
crabjar kanban comment [task-id] "Dev started at [time]. Engine: [hermes/codex/claude]"
```

For long-running work, call `crabjar kanban heartbeat [task-id] "note"` every few minutes.

**Hand off to review (Dev, after PR created):**

```bash
crabjar kanban complete [task-id] \
  --summary "PR #[number] created: [url]. Ready for review." \
  --metadata '{"pr_number":"[number]","pr_url":"[url]","next_assignee":"qa"}'
```

**Record review passed (QA):**

```bash
crabjar kanban comment [task-id] "QA passed. Preview healthy. Approval sent at [time]."
```

**Complete (Ops, after merge + healthy deploy):**

```bash
crabjar kanban complete [task-id]
crabjar kanban comment [task-id] "Merged PR #[n]. Production healthy at [time]."
```

**Block (any agent, when stalled):**

```bash
crabjar kanban block [task-id]
crabjar kanban comment [task-id] "Blocked: [what is blocking and since when]"
```

## Quick reference

| Agent | Action | Command |
|---|---|---|
| PM | Issue triaged | `crabjar kanban create(..., assignee="dev")` |
| Dev | Starting work | `crabjar kanban claim [id]` |
| Dev | Long-running work | `crabjar kanban heartbeat [id] "note"` |
| Dev | PR created | `crabjar kanban complete [id] --summary ... --metadata ...` |
| QA | Review passed | `crabjar kanban comment [id] "QA passed..."` |
| Ops | Deployed healthy | `crabjar kanban complete [id]` |
| Any | Cannot proceed | `crabjar kanban block [id]` |

## Persistence

All kanban state is stored in crabjar's state-docs system:
- Cards: `state-docs/kanban/cards/` (one `.md` per card)
- Board state: `state-docs/kanban/board.json` (current column assignments)
- History: `state-docs/kanban/history/` (all state transitions)

## Pitfalls

- Save the task ID to memory immediately after `kanban_create` — without it you cannot update or link the card later.
- Do not skip this skill. The kanban is how all agents coordinate.
- `crabjar kanban complete` is final. For partial progress, use `crabjar kanban comment` to log state.
- Blocked cards should always have a comment explaining what is blocking.
- Worker profiles should prefer `crabjar kanban` CLI calls over shelling out to other tools.

## Verification

```bash
crabjar kanban list    # shows all cards in current state
crabjar kanban stats   # summary counts per column
```

Cards appear in the correct state with accurate comment threads.
