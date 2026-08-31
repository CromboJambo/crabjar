---
id: ADR-006
title: Attempt Graph as Falsification Record
status: Accepted
date: 2026-08-30
see_also: [[ADR-005]], [[ADR-002]], [[ADR-004]]
---

# ADR-006: Attempt Graph as Falsification Record

## Status

**Accepted** (2026-08-30). Discussed and agreed in session. The design
converged from the user's speedrun framing: session playback and
undo/redo for agentic coding, like stateful emulators used by speedrunners
to learn "where they failed vs where the game makes you start." The final
reframe — the agent is a *probe*, not a *prover*, and the user's own
failure definition is deferred and layered — is the load-bearing decision.
Implementation is not started; see Consequences → Ongoing concerns for the
suggested first cut.

## Context

### The problem

An agent iterating on a coding task today re-runs everything from zero for
each new attempt: clone, build, configure, then the risky step. When the
risky step fails, the setup work is thrown away along with it, and the
failure itself is not addressable — you know it failed, not *which
condition broke*.

### The forces

1. **Solo work is two heads on one trunk.** The user works solo, but the
   agent is a second committer on the same line. The branch-vs-linear
   question is not "do I branch?" — a diligent solo dev already branches;
   that *is* what diligence is. The real question is whether the agent is a
   diligent second committer or a state generator that forks off and leaves
   two irreconcilable lines (agent-state vs user-state). The design must
   make the second one impossible.

2. **The machine may be ahead of the checkpoint, but the system must be
   aware of the diff and conditions of the attempt.** Rewind is not
   "restore a frozen snapshot" (which discards all good progress between
   checkpoint and now); it is "revert this attempt's own delta," and the
   attempt's preconditions tell you whether the revert is clean.

3. **The agent is a probe, not a prover.** It does not confirm the user's
   theory; it tests the theory and finds *why it fails*. It does not drive
   toward failure on purpose — it explores faithfully, the failure surfaces
   on its own, and the user brings the state back to the fail point.

4. **The user does not know what failure looks like yet.** The theory lives
   at a layer above the agent's observations. The user's failure definition
   is deferred — it may arrive through layers the user doesn't understand
   yet, or under constraints not understandable yet. Neither party has a
   clean failure definition at attempt time.

5. **In a simulation, the wall is the observation, not the fail point.**
   Agentic coding is already the maintainer/merge-manager simulator; the
   agent is the racecar. A race and a simulation differ on one thing — what
   the crash is for. In a race the crash ends the run, so you keep the car
   off the wall at all cost. In a simulation the crash is a *reading*: the
   rewind (Decision) prices the fail point at zero, so the agent can hit it
   as often as it wants, and the *observation* of it (diff + conditions +
   local outcome) is the durable asset — not the broken state, which is
   rewound away. The racer's instinct (stay green, avoid the crash) is the
   maintainer's enemy: an agent that avoids the wall never maps it.

### The constraints (verified on this box, 2026-08-30)

- **No `btrfs` CLI** on the host; `/tmp` is **tmpfs**. CoW snapshots are
  not a reliable mechanism for the fine tier (the review bridge's scratch
  workdir lives in `/tmp`).
- **herdr 0.8.2** is the execution substrate (ADR-002);
  `HerdrBackend::run_command` returns a typed `Receipt`
  (`{ command, output, exit_code, duration, cwd }`) — the seed of the
  attempt model.
- **ADR-005's typed stream** (append-only, monotonic ids, blocks,
  receipts, verifiers) is live in `crabjar-terminal` — the addressable
  record substrate.
- **The guard/trust substrate** already exists for promotion decisions
  (branch → trunk).

## Decision

**The attempt graph is a single-trunk git graph, and the record must
outlive the understanding.**

### The invariant

One trunk, not two. The trunk is the reconciled consensus line — the user's
line, plus whatever of the agent's work has been promoted. Every agent
attempt is a branch rooted at a specific trunk commit. That root is a
precondition of the attempt. Reconcilability is a *check*, not a hope:

> an attempt is mergeable/revertable iff the trunk hasn't moved in the
> regions the attempt touched since the attempt's root.

The agent never silently produces a parallel trunk.

### The attempt

An **Attempt** = `Receipt` (ADR-005) + `diff` + `preconditions` +
`invertible` flag + `parent` (trunk commit). The git commit graph *is* the
attempt graph: each attempt is a commit on a branch off the trunk;
merge/rebase/revert *is* the reconciliation machinery. The typed stream
(ADR-005) is the per-session log and stays untouched — no `STREAM_VERSION`
bump. The `Attempt` links a stream block/receipt to a git commit; it is a
view over the two existing structures, not a third state space.

### The burden partition

```
Agent (fully, mechanical):
  - root every attempt in the trunk
  - rebase when the trunk moves
  - record every attempt faithfully: diff + conditions + local outcome
    (exit code, verifiers) + doubt block (assumptions, blind_spots,
    last_validation, stale_after)
  - keep mapping — no stopping at the first failure, no pruning,
    no curation, no direction judgment
  - make any recorded point rewound-able

User (fully, strategic):
  - hold the theory (at a higher layer than the agent's observations)
  - judge direction — deferred; may arrive under different constraints
  - navigate the graph; bring state back to fail points
  - promote (branch → trunk) — a trust decision on the guard substrate
  - triage the bounded queue — enforced, not aspirational; see
    "The Maintainer's Contract"

Record (the medium between the layers):
  - outlasts both parties until understanding arrives
  - when the user's failure definition finally arrives, the accumulated
    graph is queryable: point at the failed attempt, see which condition
    broke
```

No overlap. The agent's diligence is *faithfulness*, not curation. The
agent carries the mechanical burden fully; the strategic one (direction)
stays with the user — and even that is deferred, because the user is also a
probe at a higher layer.

### The agent's objective: observe the wall, not stay green

The agent's objective function is to **observe the fail point**, not to keep
the build green. It is forbidden from working around the wall (cheating the
test, mocking the dependency, skipping the flaky case) — every workaround
destroys the map. The verifiers (exit code, file exists, regex) are
*readings* of the attempt, not *goals to pass*. This is probe-not-prover
made concrete as an objective: the agent drives *toward* the wall on purpose
where the user's theory predicts it, and records the observation when it
arrives.

### The report

When the agent reports, the report is **structured**: the failed attempt's
diff reference + the broken preconditions as *pointers into the graph*
(addressable — you can jump to the attempt and its conditions), with prose
as the summary on top. Per the CLI output contract, every report carries a
`doubt` block — including `blind_spots` for what the agent could not
observe (processes, network, external state). The record tags what it
couldn't resolve, so the future reader knows where to look.

### The Maintainer's Contract (forced, not aspirational)

The agent's mapping duty and the user's triage duty are coupled by a
bounded queue: producer (agent, keeps mapping) / consumer (user, triage) /
finite buffer. "Good maintainer" is not a personality trait; it is what
the mechanics make you do.

1. **Bounded triage queue.** Every attempt lands `unjudged`. The queue has
   a budget (count and/or age). When full, the agent **stops mapping** —
   new attempts are refused with a structured report: queue full, N
   unjudged, oldest X days; triage to continue. If the maintainer is
   absent, the system halts instead of accumulating garbage. A halted map
   is itself the signal that maintenance is missing. (guard pending-queue
   pattern)
2. **Promote and discard both require an annotation.** One-line direction
   judgment on promote; one-line reason on discard. Both carry a `doubt`
   block — assumptions and blind spots *of the judgment*. The CLI rejects
   a judgment without its doubt block. (CLI output contract, applied to
   the user's side)
3. **The theory is a state-doc.** The theory under test is a state-doc
   with the existing staleness tiers (fresh <7d → moldy >30d). Stale or
   expired → every agent report warns "failures may be against a theory
   you've outgrown." Re-indexing (revising the theory) resets the clock.
   (`StateDocQuerier::staleness_status()` — already built)
4. **Maintenance debt is visible.** `crabjar attempts status`: unjudged
   count, oldest age, broken conditions awaiting diagnosis, theory
   staleness, queue budget usage. The maintainer dashboard. Structured
   JSON with a `doubt` block, per the CLI output contract.

"Good maintainer" reduces to four mechanical habits: triage before the
agent halts, label every promote/discard, keep the theory doc fresh, watch
the debt metric. None can be skipped without the system visibly stopping.

### Two tiers

- **Fine tier (delta)** — git-in-the-workdir. Diffs, the graph, and
  conditions (parent tree) come from git. Surgical revert via `git
  revert`. Sidesteps both box constraints (no CoW FS needed; works on
  tmpfs).
- **Coarse tier (snapshot)** — the ephemeral VM (ADR-002 habitat):
  destroy + restore image. The safety net for the non-invertible
  (`rm -rf`, `git push`, migrations) and the non-diffable (processes,
  network, external state). An attempt's `invertible` flag decides which
  tier a rewind uses; non-invertible attempts refuse fine-tier revert.

## Options Considered

- **Frozen snapshot checkpoints (emulator save-states)**: rejected.
  Rewind = jump back to a frozen point, discarding all good progress
  between checkpoint and now. Coarse, and it destroys the record of what
  happened in between — which is exactly the learning substrate.
- **Agent as prover with direction tags** (agent labels each move
  right/wrong direction): rejected on two grounds. (1) Local verifiers
  (exit 0, file created) cannot see direction — a step can pass every
  verifier and be the wrong direction. (2) The user's own failure
  definition is deferred and layered — there is no stable direction
  signal for the agent to learn from. The agent records; the user judges,
  later.
- **Record-only, no git** (tarball diffs, no repo assumption): rejected.
  No reconciliation machinery — merge/rebase/revert are the point. Git
  gives the graph, the diffs, and the conditions for free; building that
  by hand is re-inventing git badly.
- **CoW filesystem snapshots as the fine tier** (btrfs): rejected on this
  box. No `btrfs` CLI, `/tmp` is tmpfs. Git works everywhere the workdir
  works.
- **Git-based attempt graph, record-first** (chosen): the commit graph is
  the attempt graph; the agent is a diligent second committer carrying the
  mechanical burden; the record outlives the understanding.

## Consequences

### Positive consequences

- **Surgical revert.** Undo the bad step, keep the good progress.
  `git revert` on the fine tier; the user chooses which point to bring
  state back to (rewind is user-initiated, never agent-auto).
- **Addressable failure diagnosis.** The preconditions' real job is the
  *why*: when the theory fails, point at the failed attempt and see which
  condition broke. Revert-safety is a side benefit; diagnosis is the point.
- **The record outlives the understanding.** Both parties' failure
  definitions are deferred; the record bridges the gap retroactively.
  The `doubt` block makes the record's own limits mechanical — it tags
  what it assumed and what it couldn't see.
- **Reconcilability is mechanical.** The single-trunk invariant + the
  trunk-moved check make "two irreconcilable states" impossible by
  construction, not by discipline.
- **Composable learning.** Graphed over many attempts, the accumulated
  conditions-under-which-the-theory-breaks are the map. "Where they failed"
  = the recorded attempts with conditions; "where the game makes you start"
  = the fail point the user brings state back to.
- **No new state space.** The Attempt is a view over the git graph + the
  ADR-005 stream. Both already exist and are tested.
- **Maintainer discipline is enforced by mechanics.** The bounded queue,
  annotated judgments, state-doc theory, and debt dashboard make the
  triage duties of a good maintainer visible, bounded, and impossible to
  skip silently. The system halts rather than rots.

### Negative consequences (trade-offs)

- **Diff completeness is best-effort.** A terminal command's "diff" is
  files plus processes plus network plus external state. Git captures
  files cleanly; the rest is not diffable. The `doubt` block's
  `blind_spots` carry what the record couldn't see — but the fine tier's
  diff is only as complete as the filesystem.
- **Invertibility is per-attempt.** `rm -rf`, `git push`, migrations have
  no clean local inverse. The `invertible` flag handles this, but
  classifying commands as invertible is heuristic — the coarse tier (VM
  destroy + restore) is the fallback, which is slow.
- **Git assumption on the workdir.** The fine tier requires the workdir to
  be a git repo. Workdirs that aren't (or shouldn't be) repos fall to the
  coarse tier.
- **The agent keeps mapping, which costs time and tokens.** A falsifying
  agent that finds one failure could stop; by design it keeps going until
  the user says stop or navigates, because the value is the map, not a
  single red X.

### Ongoing concerns

- **First cut: record-only.** Implement the Attempt model + git graph +
  structured report (diff ref + broken-condition pointers + doubt block)
  *without* auto-rewind. The evaluation/diagnosis record is the hard and
  genuinely new part; the revert is git doing its job and can follow.
  The fine-tier `git revert` and the coarse-tier VM restore are the second
  cut.
- **Promotion mechanics.** Branch → trunk is a trust decision on the
  guard substrate. Not designed yet — the guard's trust layers need a
  "promote attempt" action.
- **Generalization beyond git.** If the workdir is not a repo, or the
  attempt touches non-filestate, the fine tier degrades to the coarse
  tier. Re-evaluate when the habitat (ADR-002/ADR-003) VM layer is live —
  the coarse tier's home.
- **The directional judgment stays with the user, deferred.** If/when a
  goal model or the user's failure definition becomes queryable, the
  record is already shaped to accept it (attempts carry conditions and
  local outcomes; a direction label would attach as an annotation, not a
  mutation of the append-only stream).

## References

- [[ADR-005]] — typed terminal stream: the `Receipt`, blocks, verifiers,
  and the append-only stream this ADR builds on.
- [[ADR-002]] — herdr as execution substrate: `HerdrBackend::run_command`
  and the ephemeral VM that is the coarse tier's home.
- [[ADR-004]] — the glass: the attempt graph is inside the glass (a view
  over the stream + git); the git/VM concretes are outside.
- Speedrun TAS/savestate practice — the framing: a faithful frame log of
  every attempt plus rewind to any of them, where the agent has already run
  the level and hands the user the map of failure modes.
