---
name: loop-detector
description: |
  Detect when the agent is stuck in repetitive patterns (re-reading the same files,
  retrying failed commands, circling the same decision) and trigger self-correction.
  Use whenever the user mentions "loop", "stuck", "repeating", "circling", "can't
  make progress", "same error again", or when you notice yourself doing the same
  thing 3+ times without forward motion. Also trigger on your own detection of
  repetitive tool call patterns, identical grep/search results, or commands that
  keep failing with the same error.
---

# Loop Detector

Detects when the agent is stuck in a repetitive pattern and applies a weighted
escalation ladder: self-correct → re-organize → ask user. The order is not
fixed — it is chosen dynamically based on what the loop looks like and what
the user has signaled about their preferred intervention level.

## Loop Types

### 1. Command Loop
Same command (or near-identical) repeated 3+ times with the same result.
- Pattern: `cargo check` → same error → `cargo check` → same error
- Fix: Stop retrying. Diagnose the root cause. Propose a fix.

### 2. File-Reading Loop
Reading the same files or searching the same patterns repeatedly without
extracting new information.
- Pattern: `grep foo` → no match → `grep bar` → no match → `grep foo` (again)
- Fix: Mark files as "already read." Change search scope or strategy.

### 3. Decision Loop
Circling the same architectural choice without committing.
- Pattern: "Should I use X or Y?" → analyze → "maybe Z" → analyze → back to X
- Fix: Apply the "satisficing" heuristic — pick the first option that meets
  all hard constraints. Defer the rest.

### 4. Error-Recovery Loop
Attempting a fix, seeing it fail, trying a slightly different fix, repeating.
- Pattern: fix attempt 1 → error → fix attempt 2 → same error → fix attempt 3
- Fix: After 2 failed fix attempts, stop and present options to user.

### 5. Context-Bloat Loop
Accumulating context without producing output.
- Pattern: reading more files → more reads → still no concrete result
- Fix: Set a hard limit on pre-work (e.g., "read 3 files max, then act").

## Escalation Ladder

The agent chooses which rung to climb based on loop type and user preference.
Default order: **self-correct → re-organize → ask user**.

### Rung 1: Self-Correct (silent, no user intervention)

Trigger when:
- Loop is mechanical (command retry, file read)
- The fix is obvious from the evidence already collected
- User has not signaled they want to be consulted

Actions:
1. **Declare the loop** — "I notice I'm looping on X. Stopping."
2. **Break the pattern** — do something different (change tool, change scope, change angle)
3. **Commit to a direction** — pick one option and proceed

### Rung 2: Re-Organize (structural intervention)

Trigger when:
- Self-correct didn't break the loop
- The problem is structural (bad plan, wrong assumption, missing info)
- The agent needs to step back and reframe

Actions:
1. **Lay out what's been tried** — list attempts with outcomes
2. **Identify the invariant** — what's not changing? that's the real problem
3. **Propose a restructuring** — new plan, new angle, new constraint
4. **Ask for approval** to proceed with the restructure (not approval to continue the loop)

### Rung 3: Ask User (explicit intervention)

Trigger when:
- Multiple rungs have been attempted
- The decision requires user judgment (trade-offs, preferences, priorities)
- User has signaled "ask me" or "don't decide for me"
- The loop involves destructive or irreversible actions

Actions:
1. **Summarize the loop** — what you've tried, what's stuck
2. **Present options** — 2-3 concrete paths forward with trade-offs
3. **Ask a specific question** — not "what should I do?" but "should I do X or Y?"
4. **Wait** — do not proceed until the user responds

## Dynamic Weighting

The agent adjusts the escalation order based on:

### User-Configurable Override

The override is a parameter in this skill. Check locations in priority order:

1. `~/.config/opencode/AGENTS.md` under `## Loop Detection` (opencode harness)
2. `.agents/config.md` under `## Loop Detection` (project-local, any harness)
3. Inline directive from user in the current session (highest priority)

```
loop_escalation = ["ask_user", "reorganize", "self_correct"]
```

The agent must check both locations at session start. If neither exists, use
the default order. A missing override is not an error — it means the user has
no preference and the default applies.

Reverses the default — user is asked first. Absent → use default order.

### Context-Aware Adjustment
The agent adjusts dynamically based on:
- **Signal from user**: "just fix it" → bias toward self-correct. "don't decide" → bias toward ask_user.
- **Loop severity**: 3 iterations → self-correct. 5 iterations → re-organize. 8+ → ask user.
- **Task criticality**: low-stakes (formatting, refactoring) → self-correct. high-stakes (deletion, deployment) → ask user.
- **Time spent**: if the loop has consumed >5 turns, escalate one rung.

### Severity Counter

Track loop iterations internally:

| Iterations | Default Action |
|---|---|
| 3 | Self-correct |
| 5 | Re-organize |
| 8 | Ask user |
| 12+ | Explicitly notify user: "I've been stuck for 12 turns. I need direction." |

## Detection Signals

### Primary Heuristic: Planning Language Without Execution

**This is the most important signal.** If the agent produces 3+ turns of
planning/analysis language without a concrete tool call that changes state
(file write, command execution, kernel launch), it is in a planning loop.

Planning language patterns:
- "I should...", "I could...", "maybe I...", "one option is..."
- "Let me think about..." without following through
- "The approach would be..." followed by more planning, not action
- Analyzing options (X vs Y vs Z) across multiple turns with no commitment

Threshold: **3 turns of planning language without a state-changing action
triggers immediate escalation.** This caught tonight's TMA loop and the
WGMMA shared-memory loop before they hit the wall.

### Secondary Signals (agent detects)

- Same tool call with same parameters 3+ times
- Same file read within 2 turns
- Search with same pattern returning same result
- Error message repeated across different fix attempts

### Explicit Signals (user says)

- "I feel like we're going in circles"
- "stop repeating yourself"
- "just pick something"
- "what are we even trying to do?"

## Output Format

When declaring a loop, output:

```
## Loop Detected: <type>

**Pattern**: <what's repeating>
**Iterations**: <count>
**Invariant**: <what's not changing>

**Action**: <self-correct / reorganize / ask_user>
**Next**: <what you'll do instead>
```

When asking the user for direction:

```
## I'm Stuck

I've tried:
1. <attempt 1> → <result>
2. <attempt 2> → <result>
3. <attempt 3> → <result>

The invariant is: <what's not changing>

Options:
A) <option A> — pros / cons
B) <option B> — pros / cons
C) <option C> — pros / cons

Which direction?
```

## Integration with Agent Autonomy Constraints

- **Detection ≠ Authorization**: Loop detection is observation. Escalation to "ask user"
  does not execute anything — it presents options and waits.
- **Reversibility Gating**: If the loop involves destructive actions, skip to ask_user
  regardless of iteration count.
- **Confidence Decay**: Each loop iteration reduces confidence in the current approach.
  After 5 iterations, confidence drops below threshold — re-organize is mandatory.

## Boundary with Other Skills

- **`dreaming-mode`**: Loop detection is the "waking up" counterpart. When dreaming-mode
  does creative synthesis, loop-detector prevents creative spirals from becoming unproductive.
- **`bounded-agent-execution`**: Loop-detector tracks sequential turns. bounded-agent-execution
  tracks parallel fan-out. They complement each other — loop-detector catches what parallelism
  doesn't help with (circular dependencies, bad assumptions).
- **`session-backtrace`**: If a loop persists across session boundaries, session-backtrace
  can recover context. Loop-detector should check "have we tried this before?" against
  session history.
- **`attention-logger`** (forward reference): Loop-detector can query attention for
  "is this pattern recurring across multiple sessions?" — requires cross-session pattern
  detection not yet implemented. Mark as aspirational until the attention layer supports
  session-spanning queries.

## When NOT to Trigger

- The user explicitly wants iterative refinement ("try this, then tweak it")
- A command legitimately needs multiple attempts (e.g., waiting for a service to start)
- The loop is productive (each iteration reveals new information)
- The user has said "I'm handling this" or taken over

## Key Principle

A loop is not a failure — it's a signal that the current approach has diminishing returns.
The goal is not to avoid loops but to detect them early and escalate appropriately.
Every agent loops. The difference between a productive loop and a wasteful one is
**awareness + intervention**.
