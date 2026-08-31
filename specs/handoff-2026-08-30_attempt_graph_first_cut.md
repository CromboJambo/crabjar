# Session handoff: crabjar — ADR-006 attempt graph, first cut (record-only)

Companion to `specs/ADR-006_attempt_graph_falsification_record.md`. Read that
first — it is the spec; this file tells you what exists, what was rejected,
and where to start. The goal: the **record-only first cut** — the `Attempt`
model, the bounded triage queue, the structured report, and the
`crabjar attempts status` dashboard. No auto-rewind (second cut).

## What exists today (verified, not from docs)

### 1. `specs/ADR-006_attempt_graph_falsification_record.md` — ✅ Accepted
The spec. Key decisions (all in the ADR, summarized here so the fresh
session can act without re-deriving them):

- **Single trunk, not two.** The trunk is the user's reconciled line. Every
  agent attempt is a branch rooted at a trunk commit; that root is a
  precondition. Reconcilability is a check ("trunk hasn't moved in the
  regions the attempt touched"), not a hope.
- **Probe, not prover.** The agent records; the user judges direction —
  deferred, at a higher layer. No agent-side direction tags.
- **The record outlives the understanding.** Neither party knows what
  failure looks like yet. The `doubt` block (assumptions, blind_spots,
  last_validation, stale_after) makes the record's own limits mechanical.
- **The Maintainer's Contract (forced, not aspirational).** Bounded triage
  queue (full → agent stops mapping); promote and discard both require an
  annotated judgment with a doubt block; the theory is a state-doc
  (existing staleness tiers); `crabjar attempts status` shows maintenance
  debt.
- **Drive the road, not aim for the wall.** Full collision, not near miss;
  not deliberately headlong. The agent is forbidden from working around the
  fail point (cheating tests, mocking deps, skipping flaky cases).
  Verifiers are *readings*, not *goals to pass*.
- **The co-driver tenses before, not after.** The agent emits **approach
  warnings** ("the theory predicts a break at X, I'm getting close") before
  the full collision.
- **Two tiers.** Fine = git-in-the-workdir (surgical revert). Coarse =
  ephemeral VM destroy + restore (non-invertible, non-diffable). The
  `invertible` flag on an attempt decides the tier.
- **No new state space.** The `Attempt` is a *view* over the git graph +
  the ADR-005 stream. It links a stream block/receipt to a git commit.
  `STREAM_VERSION` is NOT bumped (stays 2).

### 2. `crates/terminal/` (package `crabjar-terminal`) — ✅ the substrate
- `src/stream.rs` (415 LoC) — `TerminalEvent` (Prompt/Command/Output/Raw,
  monotonic ids, per-event `at`), `SessionStream` (`push`, `push_receipt`,
  `blocks`), `Block` (+ `Block::receipt`), `Receipt
  { command, output, exit_code, duration, cwd }`, `STREAM_VERSION = 2`.
- `src/verifiers.rs` (336 LoC) — `exit_code`, `file_exists`, `regex_match`,
  `json_path` as pure functions of `(Receipt, expectation)` →
  `VerifierResult { verifier, passed, detail }`.
- `src/copy_paste.rs`, `src/session_record.rs`, `src/recording.rs` —
  selection, versioned JSONL, asciinema serializer. All live, all tested.
- `src/herdr_exec.rs` — `HerdrBackend::run_command` → typed `Receipt` via
  the structured round-trip (markers + `pane wait-output --regex`).
- `src/lib.rs` (346 LoC) — `TerminalSession` + `TerminalManager`.
- **Headroom for the 500 LoC gate:** `stream.rs` 415, `lib.rs` 346. The new
  `attempts.rs` must be its own file (it will be); do not grow `stream.rs`
  past 500.

### 3. `axum-mux/` (package `vm-bridge`) — ✅ live review bridge
- `src/review.rs` (125 LoC) — `--review <port> <session> <cmd...>` mode;
  the first real `TerminalEvent` producer. Verified live (commit `dc28994`):
  3 commands → 9 events → joined reviewer, zero drops.
- `examples/review-client.rs` (115 LoC) — the unintrusive reviewer.
- `src/terminal_relay.rs` (446 LoC) — thin transport; `publish_event()` is
  the producer entry point.

### 4. CLI — root `src/`
- Existing subcommands follow the CLI output contract: structured JSON on
  stdout, `success: true/false`, and a `doubt` block on derived output
  (assumptions, blind_spots, last_validation, stale_after). `state list`,
  `state staleness`, `knowledge` are the reference shapes.
- `crabjar attempts status` (this cut) must match that contract exactly.

### 5. Compilation status
- `cargo check --workspace` — ✅ warning-free.
- `just test-crate axum-mux` — ✅ 11 pass.
- `just test-crate crates/terminal` — ✅ pass (verify before starting;
  the substrate is green as of `dc28994`).

## What was NOT done (rejected)

- **Frozen snapshot checkpoints (emulator save-states)**: Rejected because
  rewind-to-snapshot discards all good progress between checkpoint and now,
  and destroys the record of what happened in between — the learning
  substrate. Correct path: per-attempt delta (git) + the append-only
  record.
- **Agent direction tags** (agent labels moves right/wrong): Rejected
  because local verifiers can't see direction, and the user's own failure
  definition is deferred and layered. Correct path: the agent records; the
  user judges, later, via the annotated promote/discard.
- **Record-only without git** (tarball diffs, no repo assumption):
  Rejected because merge/rebase/revert are the reconciliation machinery and
  git provides the graph, diffs, and conditions for free.
- **CoW filesystem snapshots as the fine tier** (btrfs): Rejected on this
  box — no `btrfs` CLI, `/tmp` is tmpfs. Git works everywhere the workdir
  works.
- **Auto-rewind in the first cut**: Deferred to the second cut. The
  record (Attempt + queue + report + dashboard) is the hard and new part;
  `git revert` is git doing its job.
- **A third state space for attempts**: Rejected. The Attempt is a view
  over the git graph + the ADR-005 stream, not a parallel store.

### ⚠️ Open design point: where approach warnings live
The ADR requires approach warnings, but does not fix their wire form.
Options: (a) a new `TerminalEvent` variant — this changes the event
vocabulary and forces `STREAM_VERSION` 2 → 3 with a migration; (b) an
attempt-level annotation (part of the Attempt record / structured report,
emitted via the relay's existing publish path) — no stream change.
**Lean: (b) for the first cut** — it keeps `STREAM_VERSION` at 2 and keeps
the stream vocabulary stable. If (a) is chosen instead, it is a separate
decision with its own migration test, not a side effect of this cut.

## Remaining work (in priority order)

### 1. `Attempt` model — `crates/terminal/src/attempts.rs` (new file)
The core type. Shape per the ADR:

```rust
pub struct Attempt {
    pub receipt: Receipt,          // ADR-005: command, output, exit_code, duration, cwd
    pub parent: String,            // trunk commit the branch is rooted at (precondition)
    pub diff: String,              // the attempt's delta (git diff text or structured ref)
    pub preconditions: Vec<Condition>,
    pub invertible: bool,          // fine tier (git revert) vs coarse tier (VM)
    pub intent: String,            // "I was trying to hit X, I got Y" — dodge detection
    pub status: AttemptStatus,
}

pub struct Condition {
    pub name: String,
    pub expected: String,
    pub actual: Option<String>,    // None = not yet observed
    pub broken: bool,
}

pub enum AttemptStatus {
    Unjudged,
    Judged(Judgment),
}

pub struct Judgment {
    pub annotation: String,        // one-line direction judgment (promote) or reason (discard)
    pub doubt: Doubt,              // assumptions, blind_spots, last_validation, stale_after
}
```

The `intent` field is in scope for this cut — it is cheap to add now and
expensive to retrofit after attempts accumulate (it is the seed of
mechanical dodge-detection). `Doubt` should be a shared type if one already
exists in the workspace (check `guard/` and the CLI contract helpers before
defining a new one). Unit tests: construction, `broken` condition
detection, judgment round-trip through serde.

### 2. Bounded triage queue — same file or `queue.rs` (watch the 500 LoC
gate; split if `attempts.rs` approaches it)
- `TriageQueue { attempts: VecDeque<Attempt>, budget: usize }`
- `push` → lands `Unjudged`; when `len >= budget`, `push` is **refused**
  with a structured refusal (queue full, N unjudged, oldest age) — the
  agent stops mapping.
- `judge(id, Judgment)` — promote or discard; requires the annotation +
  doubt block (reject a judgment without its doubt block).
- `oldest_age()` for the dashboard.
- Unit tests: budget refusal, judgment validation, oldest-age computation.

### 3. `crabjar attempts status` — root `src/` CLI
Structured JSON per the CLI output contract, with a `doubt` block:
unjudged count, oldest age, broken conditions awaiting diagnosis, queue
budget usage, theory staleness (wire to
`StateDocQuerier::staleness_status()` if the theory doc exists; otherwise
report `null` with a blind spot noting the theory doc is not yet created).
Follow the existing subcommand registration pattern (see `state` and
`knowledge` subcommands). Snapshot tests if the project's `insta` setup
covers CLI output (check `tests/snapshots/`).

### 4. Structured report — `attempts.rs`
`report(attempt) -> serde_json::Value`: the failed attempt's diff reference
+ broken preconditions as **pointers into the graph** (addressable — the
consumer can jump to the attempt and its conditions), prose summary on top,
`doubt` block per contract. This is what the agent emits instead of freeform
"it failed."

### 5. Approach warnings — annotation form (see ⚠️ above)
An `ApproachWarning { predicted_break: String, proximity: ... }` annotation
on the attempt, surfaced in the structured report and the dashboard. No
stream change in this cut.

### 6. (Second cut — do NOT start) Fine-tier `git revert` + coarse-tier VM
restore. The `invertible` flag and the tier decision are recorded now;
executing the rewind is the next session.

## Key implementation decisions

- **Attempt is a view, not a store.** It links a stream block/receipt to a
  git commit. Do not build a parallel persistence layer; `SessionRecord`
  (versioned JSONL) is the faithful on-disk form for the stream side.
- **`STREAM_VERSION` stays 2.** No new `TerminalEvent` variants in this cut
  (see the ⚠️ open point).
- **Refusal is a structured value, not a panic.** A full queue returns the
  refusal to the caller (the agent halts); it never panics or silently
  drops.
- **Judgment without doubt is rejected.** The CLI contract's `doubt` block
  applies to the user's side too — that is the point of the Maintainer's
  Contract.
- **`intent` is recorded, not enforced, in this cut.** Mechanical
  dodge-detection is a later decision; the field exists so the data is
  there when it is.

## What not to do

- **Don't implement auto-rewind.** Second cut. The first cut is record-only.
- **Don't bump `STREAM_VERSION`.** The stream vocabulary does not change
  this cut.
- **Don't add a third state space.** No new database, no new file format
  beyond what `SessionRecord` already provides.
- **Don't grow `stream.rs` past 500 LoC.** New code goes in `attempts.rs`
  (and `queue.rs` if needed).
- **Don't generalize to wezterm/zellij or the VM tier.** Herdr/git only,
  per the ADR's prove-on-one-substrate discipline.
- **Don't rename `axum-mux`/`vm-bridge` as a side effect** (ADR-005 item 7,
  still open, still separate).

## Environment

- **Narrow-scope tests:** `just test-crate <dir>` / `just check-crate <dir>`
  / `just clippy-crate <dir>` — pass a crate *directory* (e.g. `guard`,
  `crates/terminal`, `axum-mux`). Package names ≠ directory names for
  17/25 crates; `crates/terminal` is package `crabjar-terminal`,
  `axum-mux` is package `vm-bridge`.
- **`just` is at `~/.cargo/bin/just`** — `export PATH="$HOME/.cargo/bin:$PATH"`
  first in a fresh session.
- **500 LoC rule** is a CI gate (`just module-sizes-check`).
- **Git remote is SSH-only** (`git@github.com:CromboJambo/crabjar.git`) —
  never switch to HTTPS; if push fails, leave the commit local and tell the
  user to push.
- **CI has no `just`** — inline bash in CI jobs.
- **Box constraints:** no `btrfs` CLI; `/tmp` is tmpfs (CoW snapshots
  unreliable — git-in-the-workdir sidesteps both).
- **herdr 0.8.2** server runs locally; `crabjar-terminal` examples that
  need it: `cargo run -p crabjar-terminal --example herdr-stream-spike`.
- **Commit style:** one commit per logical change; the ADR-006 commits
  (`9b1cb92`, `f5e8781`, `19f42c8`, `1463f39`, `1fc0a96`) are the
  reference for message tone.

## Files to review (with purpose)

- `specs/ADR-006_attempt_graph_falsification_record.md` — the spec; read
  first. Every shape in this handoff defers to it.
- `crates/terminal/src/stream.rs` — the `Receipt` and `SessionStream` the
  `Attempt` wraps; check the exact field names before writing `attempts.rs`
  (the naming-collision rule: parameters must match what the type actually
  has).
- `crates/terminal/src/verifiers.rs` — `VerifierResult` and the verifier
  signatures; the structured report composes these as readings.
- `crates/terminal/src/lib.rs` — where `attempts` gets re-exported; check
  the existing module list and export pattern.
- Root `src/` CLI (find via `ls src/` — the `state` subcommand is the
  reference) — where `attempts status` registers; match its JSON output
  shape and doubt block exactly.
- `memory/src/state_docs/models.rs` — `StalenessStatus` +
  `StateDocQuerier::staleness_status()` for the theory-staleness field in
  the dashboard.
- `axum-mux/src/review.rs` — the existing producer; the second cut will
  wire attempts into it, so keep the `Attempt` shape compatible with how
  `review.rs` appends receipts to the stream.
