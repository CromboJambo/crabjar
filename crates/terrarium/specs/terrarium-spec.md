# The Terrarium — Spec

> A third skin over the crabjar habitat contract. Where `flock` renders agents
> as sheep on a farm and `dagr` renders the contract as a DAG, the terrarium
> renders it as a **living isometric habitat of hermit crabs** — crabs that
> move things around, work at terminals, dig, and sew.

Status: **Phase 0 built & tested** (headless simulator, 21 tests, clippy clean).
Stable architecture locked, taste layer parametric. Companion to `crabjar`'s
`habitat contract` producer (ADR-006 territory).

---

## 1. Vision

An isometric terrarium where each **hermit crab** is a unit of work and the
habitat is the shape of the run. Crabs do *things* that map to what the
contract says is happening:

- a crab **at a terminal, typing** = an attempt is `running`
- a crab **carrying a shell across the floor** = an attempt moving between
  stations (queued → running → done)
- a crab **digging** = a `failed` attempt entering rework
- a crab **sewing / stitching** = the act of assembling the contract itself
- a crab **stuck, waving** = `blocked` / `needs_user` — waiting on a human

The habitat is *spatial*, but the spatial layer is **not load-bearing** (see
§7 phasing). The load-bearing layer is the *interpretive mapping* — how
abstract work state becomes concrete crab behavior.

### The design mandate

> "user builds the abstractions of their abstractions based on taste"

This is the whole point. Two levels of abstraction:

- **Level 1 — the abstraction:** the *behavior vocabulary* (§4). A fixed,
  stable set of things a crab can *do* (work, dig, sew, carry, inspect, …).
  This is the contract the renderer honors.
- **Level 2 — the abstraction of the abstraction:** the *mapping* (§5). How a
  given contract state (task state × evidence tier × condition) is *assigned*
  to a behavior. This is **taste**, and it is **data, not code**.

You edit the mapping to taste. The renderer is a pure interpreter of that
mapping. No renderer code changes when your taste changes.

This is the same "generic over wire representation, not over versions"
principle that shaped the contract: the *vocabulary* is the stable substrate,
the *assignment* is disposable and swappable.

---

## 2. The one invariant

The terrarium is a **pure consumer** of the habitat contract.

```
crabjar ──(habitat contract)──▶ run.json ──▶ terrarium (this)
                                     │
                                     └──▶ dagr (existing skin)
                                     └──▶ flock (agent.list skin)
```

- **In:** `run.json` (dagr v3 contract), produced by `crabjar habitat contract`.
  The terrarium never writes it, never mutates it, never re-derives it.
- **The glass:** the terrarium lives *outside* the crabjar workspace and looks
  *in*. It reads the contract as data. It never names a crabjar-internal type,
  never imports a crabjar crate, never imports the `dagr` renderer.
- **Liveness (optional):** the herdr socket (`agent.list` / `events.subscribe`)
  is a *hint* channel only — e.g. "this attempt's VM pane is alive / closed" —
  exactly as `dagr` demotes it. The contract file remains the sole authority on
  run-state. A missing socket must not blank the terrarium.

Consequences:
- The terrarium is a **separate project** (`/home/crombo/projects/terrarium`),
  not a crabjar workspace member. The 500-LoC CI gate does not apply to it; the
  glass principle does.
- It can be deleted, rewritten, or re-skinned without touching crabjar.
- It runs standalone (`terrarium view run.json`) *and* as a herdr plugin (pane,
  watching the same `.dagr/run.json` the dagr pane watches).

---

## 3. What it consumes

The dagr v3 contract, already produced and `dagr check`-clean. Top-level:

```
run.json
├── run        {id, title, started_at, now}
├── projects[] {id, title}              ← habitat zones
├── tasks[]    {id, project, title, state, attempts[]}
├── attempts   (nested in tasks) {state, outcome{result,evidence,reason}, ...}
└── events[]   {at, task, message}      ← W207: must be time-ordered
```

The terrarium reads a *subset* of this: `projects` (→ zones), `tasks` (→
stations + their state), `attempts` (→ the crabs + their behavior), `events`
(→ optional ambient narration / a "tidings" ticker). It does **not** need the
full DAG edge structure — the spatial scene replaces the graph edges with
*physical movement*.

Evidence tiers (`verified` / `reported` / `heuristic` / `asserted`) map to a
crab's *certainty* — a visual signal (tint, posture, animation detail), not a
behavior. See §5.

---

## 4. The behavior vocabulary (Level 1 — stable)

The fixed set of things a crab can do. This is the contract the renderer
honors and the vocabulary the mapping (§5) draws from. Deliberately small and
physical — each is a thing a hermit crab plausibly does in a terrarium.

| behavior  | physical read                       | default station |
|-----------|-------------------------------------|-----------------|
| `wait`    | crab still, shell on back, breathing| queue           |
| `carry`   | crab walking, item on its back      | (in transit)    |
| `work`    | crab at a terminal, typing          | terminal        |
| `inspect` | crab peering with a magnifier       | review          |
| `deliver` | crab happy, handing off an item     | done            |
| `dig`     | crab digging / reworking            | rework          |
| `sew`     | crab stitching pieces together      | loom            |
| `stuck`   | crab wedged, waving for a human     | blocked         |
| `rest`    | crab asleep in its shell            | (done/quiet)    |

Notes:
- **`carry`** is the spatial glue — it's how an attempt *moves* between
  stations. A task whose state changes re-routes its crab: it `carry`s from
  the old station to the new one. This is "moving things around."
- **`sew`** is reserved for the *meta* act — the producer stitching the
  contract. It's optional flavor, not tied to a task state.
- The vocabulary is **extensible by config, not code**: a new behavior is a new
  entry in §5 with a new animation name. The renderer treats unknown behaviors
  as `wait` (graceful) and logs a warning — never crashes on a taste change.

This is the stable "abstraction." Everything below it is taste.

---

## 5. The mapping layer (Level 2 — taste, data not code)

A declarative file — `behaviors.toml` — that assigns contract state to
behavior. **This is the file you edit to taste.** The renderer is a pure
function of `(run.json, behaviors.toml, width)`.

### The effective state (what the mapping keys on)

The mapping does **not** key on `task.state` alone — that would be ambiguous
(a triage task is always `state: review` whether its attempt passed or failed).
It keys on the **effective state**, a single derived value per crab:

```
effective_state(task, latest_attempt):
  if task.needs_user                    → "blocked"     # guard: always waves for a human
  elif task has a latest attempt:
       if attempt.state == "running"    → "running"
       elif attempt.outcome.result in
            {"done","pass"}             → "done"
       elif attempt.outcome.result in
            {"failed","fail","error"}   → "failed"
       else                             → "review"      # attempt settled, awaiting judgment
  else:  # no attempt yet
       task.state                       # "queued" | "blocked" | "review" | …
```

So the canonical effective-state set is:
`queued · running · review · done · failed · blocked`.
The mapping table assigns each to a behavior. This is the *one* place state
semantics live — the renderer and the simulator both call this same function,
so taste and code can never drift apart.

### The file

```toml
# behaviors.toml — the taste layer. Edit freely; no renderer code changes.
schema_version = 1

# effective_state → behavior (the core mapping; §5 "effective state")
[behavior]
queued  = "wait"
running = "work"
review  = "inspect"
done    = "deliver"
failed  = "dig"
blocked = "stuck"

# evidence tier → certainty signal (tint + posture), NOT a behavior
[evidence]
verified  = { tint = "green",  posture = "upright" }
reported  = { tint = "amber",  posture = "neutral" }
heuristic = { tint = "orange", posture = "hunched" }
asserted  = { tint = "gray",   posture = "shell-up" }

# project → habitat zone (spatial placement; see §6)
[zone]
triage = { label = "Triage Burrow", x = 0, y = 0 }
guard  = { label = "Guard Reef",    x = 4, y = 0 }
theory = { label = "Theory Grotto", x = 8, y = 0 }

# station layout within a zone (optional; defaults derive from behaviors)
[station]
terminal = { glyph = "▛", label = "terminal" }
rework   = { glyph = "⌗", label = "dig site" }
loom     = { glyph = "✂", label = "sewing" }
```

Semantics the renderer enforces regardless of taste:
- **Unknown effective state / behavior** → `wait` + warning (never crash).
- **Unknown `evidence`** → `reported` defaults.
- **`needs_user: true`** (guard pending) forces `stuck` regardless of state —
  a blocked guard action always waves for a human.
- **`reason`** (the *why* of a failure) is surfaced as the crab's "thought
  bubble" / tooltip on select, never dropped. The terrarium shows *why* a crab
  is digging, not just that it is.
- **Shell age** = the latest attempt's `n` (per-task identity, §9.3). `n=1` is a
  fresh shell; each rework adds weight.

The mapping is **versioned data** (`schema_version` field), so taste changes
are diffable, reviewable, and swappable per-user — the same way the contract
treats version diffs as data descriptors, not parallel implementations.

---

## 6. The scene model

The spatial layer. Isometric, 2:1 tiles, rendered on ratatui's 2D cell grid via
the standard iso transform:

```
screen_col = (x - y) * (tile_w / 2) + origin_col
screen_row = (x + y) * (tile_h / 2) + origin_row
```

- **Zone** = a project. A labeled region of the floor.
- **Station** = a fixed point in a zone (terminal, dig site, loom, queue).
- **Crab** = an attempt (or, when a task has no live attempt, the task itself).
  A crab has: a home zone (its project), a current station, a behavior (§5),
  a certainty tint (evidence), and an item it may be carrying.
- **Movement** = a `carry` animation interpolating the crab from old station to
  new when the task state changes. This is the "moving things around."

The scene is **derived, not stored** — it is recomputed each frame from
`(run.json, behaviors.toml)`. There is no separate spatial state to keep in
sync; the contract is the single source of truth and the scene is a pure
projection of it. (Same purity as the dagr renderer: pure function of
`(contract, width)`.)

Depth sorting: painter's algorithm on `(x + y)`, so crabs/stations occlude
correctly. Multi-cell pixel-art sprites (a crab is ~2×2 tiles), same approach
as `flock`'s sheep.

---

## 7. Phasing (spatial is NOT load-bearing)

Per your call — "it does not need to be spacial right from the start." The
behavior-mapping layer is the value; the isometric rendering is the skin on
top of it. So we build the interpreter first, the pixels later.

- **Phase 0 — the behavior simulator (headless, no rendering).**
  A pure function `(run.json, behaviors.toml) → Vec<CrabEvent>` where a
  `CrabEvent` is `{crab_id, behavior, station, tint, reason?, at}`. Fully
  unit-testable, zero TUI. This proves the mapping layer is correct and
  taste-swappable *before* a single pixel exists. **This is the first
  deliverable.** It doubles as the integration test for the whole concept.
- **Phase 1 — the minimal TUI (lightly spatial).**
  Crabs as single glyph + behavior verb on a flat grid (not isometric yet).
  Zones as labeled rows. A "tidings" ticker from `events[]`. Proves the
  frame loop, file watching, and herdr-socket liveness hints.
- **Phase 2 — the isometric terrarium.**
  2:1 iso transform, pixel-art crabs, `carry` movement interpolation, depth
  sorting, evidence tints, thought-bubbles on select. The full vision.

Each phase is a working, shippable artifact. Phase 0 is testable today with
the real `run.json` we already produce.

---

## 8. Tech stack & substrate reuse

- **Rust**, `ratatui` (TUI), `crossterm` (terminal) — same stack as both
  `flock` and `dagr`, so the ecosystem is proven.
- **herdr plugin** form factor (consistent with the other two skins):
  `herdr-plugin.toml` + binary + `[[panes]]` entrypoint, opened via
  `herdr plugin pane open --plugin terrarium --entrypoint terrarium`.
  Also runnable standalone: `terrarium view run.json [--width N]`.
- **Contract consumption:** read `run.json` (the same file the dagr pane
  watches, at `<workspace>/.dagr/run.json`), watch it (notify or mtime poll),
  re-derive the scene each change.
- **Substrate reuse (reference, not dependency):**
  - `flock` is the **reference implementation for pixel-art-on-a-grid** —
    its `model/` (sprites + procedural floor) and `ui/farm.rs` (ratatui canvas)
    are the techniques to borrow for Phase 2. We do *not* depend on flock; we
    learn from it.
  - `dagr` is the **reference for contract-driven purity** — its
    "contract file is sole authority, renderer is a pure function of
    (contract, width), socket is liveness-only" is the exact architecture we
    copy.

---

## 9. Decisions (locked) & open taste

**Locked (settled 2026-09-01):**

1. **First artifact: standalone binary**, wrapped as a herdr plugin in Phase 2.
   `terrarium view run.json` runs on its own; the plugin form factor comes later.
2. **Fresh project, borrow flock's techniques** — do *not* fork flock. Keeps the
   glass clean; flock is a reference for pixel-art-on-a-grid, not a base to mod.
3. **Crab identity = per task.** The same crab persists across a task's attempts
   and *ages its shell* with each rework (more `n` = older, heavier shell). This
   is the "hermit crab" read: the shell is the identity. The *current* attempt is
   what the crab is *doing*; the task is *who* it is.

**Open taste (deferred, not blocking Phase 0):**

4. **The `sew` meta-act.** Dedicated "loom" crab that stitches the contract when
   `habitat contract` runs (a living producer indicator), or pure flavor?
5. **Scale / crowding.** Cap visible crabs and aggregate the rest, or let the
   floor fill? (Real queue is small — 3 seeded — but a busy run could have dozens.)
6. **`carry` interpolation cadence.** When a state flips, how fast does the crab
   walk the new station? (Phase 2 concern.)

---

## 10. Non-goals (for now)

- No crabjar workspace membership. No crabjar crate imports. No dagr renderer
  imports. (The glass.)
- No spatial simulation / physics. Crabs move along a path; they don't collide
  or navigate dynamically.
- No multiplayer / networked terrarium. Single local viewer.
- No write-back. The terrarium never influences crabjar state.
