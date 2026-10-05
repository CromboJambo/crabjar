# Session handoff: crabjar habitat — close the loop, then skin it

**STATUS (2026-09-03).** The producer side is DONE and committed in crabjar
(`289a59e` + `e6d62f0`). The renderer side exists as a **separate project**
at `/home/crombo/projects/terrarium` with Phase 0 built & tested (21 tests,
clippy clean). This handoff is the spec for the next moves: wire the two ends
together, build the minimal TUI, then consolidate the three habitat codebases
into one glass-clean renderer.

Read this file first, then `specs/ADR-006_attempt_graph_falsification_record.md`
(producer-side authority) and `/home/crombo/projects/terrarium/specs/terrarium-spec.md`
(renderer-side authority — the phasing, behavior vocabulary, and taste layer
are all locked there; do not re-litigate them).

## The three habitat codebases (verified 2026-09-03)

| Codebase | Location | Role | State |
|---|---|---|---|
| Producer | `crabjar/src/habitat_contract.rs` | `crabjar habitat contract` → dagr v3 `run.json` | ✅ committed; projects triage queue, guard pending queue, theory staleness |
| Renderer v1 (demo) | `crabjar/apps/terrarium/` (2,669 LoC) | isometric demo + herdr scripts | ⚠️ disposable world model; salvageable pixels |
| Renderer v2 (the real one) | `/home/crombo/projects/terrarium` | pure consumer of `run.json`; Phase 0 headless simulator | ✅ 21 tests passing; `terrarium view run.json` works |

**The consolidation direction is locked by the renderer spec, not a new
decision:** the contract file is sole authority; the renderer is a pure
function of `(run.json, behaviors.toml, width)`; no crabjar imports (the
glass). Renderer v2 wins. From renderer v1 only the *pixels* are salvageable
(iso transform in `render_isometric.rs`, sprite work); its world model
(`GameWorld`/`Entity` vs `World`/`Crab` — two models coexist) and its fake
`plugin.rs` state (the herdr demo never showed the real contract-driven
world) are disposable.

## What exists today (verified, not from docs)

### Producer side (crabjar, `main`)

- `src/habitat_contract.rs` (~350 LoC): `build_contract()` is pure;
  `read_pending_actions(guard_db)` and `read_theory_status(db_path, theory)`
  are the only I/O seams. Evidence tiers are honest — a terminal receipt maps
  to `reported`, not `verified`.
- Emission: `.dagr/run.json` (gitignored); `attempts.jsonl` triage queue data
  (gitignored). See commit `e6d62f0`.
- ADR-006 fine tier is live (`crabjar attempts rewind`); **coarse tier is
  still deferred** — refused with `coarse_tier_not_live`; its home is the
  habitat VM layer, which does not exist yet.

### Renderer side (separate project)

- `/home/crombo/projects/terrarium`: `src/{contract,mapping,simulator}.rs` +
  `main.rs`. `cargo test --all-targets` → 21 passed.
- `behaviors.toml` — the taste layer (spec §5), versioned data, editable
  without renderer code changes.
- CLI: `terrarium view [contract] [--behaviors file]`, default contract path
  `.dagr/run.json`. Plain-text scene output; no TUI yet.
- **Never fed a real `run.json` end-to-end.** Phase 0 was tested against
  fixtures. This is the gap.

## The plan

### Phase 1 — Close the loop (first deliverable)

Wire the *real* producer output into the simulator. Today the two ends have
never met in one pipeline.

- Run `crabjar habitat contract` in crabjar, then
  `terrarium view .dagr/run.json`, and verify the rendered crab roster
  **matches reality**: each unjudged attempt in the triage queue is a crab at
  its station with the right behavior; each pending guard action renders as a
  `stuck` crab (the `needs_user` → forced-`stuck` rule, spec §5); theory
  staleness shows up.
- Add an integration test in the terrarium project that runs against a
  committed sample `run.json` captured from the real producer (not a hand-made
  fixture — capture it, commit it under `testdata/`). This is the integration
  test for the whole concept and guards the wire contract between the two
  repos.
- **Acceptance:** one command chain (`crabjar habitat contract && terrarium
  view .dagr/run.json`) produces a scene whose crab count and behaviors are
  exactly derivable from `attempts.jsonl` + the guard pending queue. Any
  mismatch is a bug in one of the two repos — fix it at the source, not by
  special-casing in the renderer.

### Phase 2 — Minimal TUI (lightly spatial)

Per spec §7: crabs as single glyph + behavior verb on a flat grid (not
isometric yet), zones as labeled rows, tidings ticker from `events[]`.

- File watching on `.dagr/run.json` — mtime poll is fine; re-derive the scene
  each change (the scene is derived, not stored).
- Herdr plugin form factor: `herdr-plugin.toml` + binary entrypoint, opened
  via `herdr plugin pane open`. Standalone mode stays.
- Liveness hint via herdr socket (`agent.list` / `events.subscribe`) demoted
  exactly like dagr: a missing socket must not blank the scene. The contract
  file remains sole authority on run-state.
- **Acceptance:** the terrarium pane in herdr updates live when
  `crabjar habitat contract` re-emits, with no manual refresh.

### Phase 3 — Isometric skin + consolidation

- Port the salvageable pixels from `crabjar/apps/terrarium/src/render_isometric.rs`
  (2:1 iso transform, painter's-algorithm depth sort) into the terrarium
  project; borrow flock's pixel-art-on-grid techniques per spec §9.2 (reference,
  not dependency). Evidence tints, thought-bubbles on select (the `reason`
  field is never dropped), shell age from attempt `n`.
- Then move the project **into crabjar as a workspace member** (suggested:
  `apps/habitat`) and delete `apps/terrarium`. This simultaneously resolves
  the two-world-models wart (only one world model survives: the contract
  projection) and retires the fake plugin state.
- The glass holds even inside the workspace: the renderer crate must not
  import any other crabjar crate — it reads a JSON file, that's the whole
  coupling. If that constraint starts biting (e.g. we want typed contract
  structs shared), extract a small `habitat-contract` types crate instead of
  reaching across crates.
- **Acceptance:** one workspace member renders the real habitat isometrically;
  `apps/terrarium` is gone; `cargo check --workspace` clean; herdr pane shows
  the live scene.

## Deferred (not blocking)

- **ADR-006 coarse tier** — ephemeral VM destroy/restore. Needs the habitat
  VM layer (ADR-002/003 territory), which doesn't exist. Keep refusing with
  `coarse_tier_not_live` until then.
- **Open taste items** from the renderer spec §9: the `sew` meta-act (loom
  crab as living producer indicator?), crowding cap vs fill-the-floor,
  `carry` interpolation cadence. Decide these in Phase 3, not before.
- **HA physical-truth seam** (ADR-003) — `host/host-mqtt` exists; the habitat
  consuming real physical state is a later layer.

## Pitfalls

- **Two repos, one contract.** The dagr v3 wire format is the only coupling
  between crabjar and the terrarium project. If Phase 1 reveals a mismatch,
  fix it in the producer or the renderer's `contract.rs` — never with
  renderer-side special cases keyed to crabjar internals.
- **Do not re-litigate the renderer spec.** Phasing, vocabulary, taste layer,
  and the locked decisions (standalone-first, fresh project not a flock fork,
  per-task crab identity) are settled. This handoff adds nothing to them.
- **Renderer v1 is a trap.** It compiles, it demos, it looks real — but its
  herdr demo rendered fake state. Treat `apps/terrarium` as an asset shelf
  (iso math, sprites), not as the base to extend.
- **500-LoC gate applies once the renderer is a workspace member.** The
  separate project is exempt today; plan module splits (`contract.rs`,
  `mapping.rs`, `simulator.rs` + future `render*.rs`) so the move doesn't
  trip CI.
- **The producer's evidence tiers are deliberately conservative.** A terminal
  receipt is the agent's own report → `reported`. Do not "improve" this by
  upgrading tiers in the renderer; the tint system exists to make that
  distinction visible.

## Where to start

1. Phase 1 first — it's a half-day of work and it de-risks everything else:
   if the real `run.json` doesn't parse or maps wrong, every later phase
   inherits the bug.
2. Capture the real `run.json` into terrarium's `testdata/` before any
   renderer changes, so the integration test pins today's producer output.
3. Verification commands:
   - crabjar: `cargo check --workspace && cargo test --workspace`
   - terrarium: `cd /home/crombo/projects/terrarium && cargo test --all-targets && cargo clippy -- -D warnings`
   - end-to-end: `crabjar habitat contract && terrarium view .dagr/run.json`
