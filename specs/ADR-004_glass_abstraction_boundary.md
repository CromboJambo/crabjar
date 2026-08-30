---
id: ADR-004
title: The Glass — Abstraction Boundary Orientation
status: Proposed
date: 2026-08-30
see_also: [[ADR-002]], [[ADR-003]]
---

# ADR-004: The Glass — Abstraction Boundary Orientation

## Status

**Proposed**. This ADR pins crabjar's abstraction boundary — "the glass" — and
the orientation rule that follows from it: every agent (internal or fleet)
must know which side of the glass it is on, and face the right direction when
it shouts orders. Ready to flip to **Accepted**; the decision is a course
change the user directed on 2026-08-30.

## Context

The trigger was a small footgun: `cargo test -p guard` fails because the
directory is `guard` but the package is `crabjar-guard`. It is systemic — 17 of
25 workspace crates have a directory name that does not match their package
name — and AGENTS.md's "narrow scope" hint (`cargo test -p <crate>`) reads
`<crate>` as the directory.

But the footgun is a symptom. The user asked the deeper question: *which side
of the glass is a good abstraction?* The answer falls out of dependency
direction plus crabjar's standing design preference ("generic over the wire
representation, not over versions"):

- The **stable, generic, depended-on substrate** is the thing everything else
  depends on — `crabjar-host-core` (event bus, WorkItem, plugin API),
  `crabjar-guard` (the exec pipeline). That is the inside of the glass.
- The **concrete** — integrations, wire representations, version-specific
  behavior — is disposable and lives on the other side: `host-graph`
  (Microsoft Graph), `host-mqtt` (MQTT/HA), `host-screen` (display protocols).

The operational consequence the user named: *fleet agents and internal agents
need to know if they are inside looking out or outside looking in, and face the
right direction when they shout orders.* Without that orientation, agents couple
across the glass. The counterexample already in the tree is `vm-bridge`
(directory `axum-mux/`, no lib target, WASM-only): `host-screen` — an
abstraction over display protocols — depends on it, so the concrete has welded
*through* the glass to another concrete. That is the failure mode the rule
exists to prevent.

Forces at play:

- **Dependency direction is the ground truth.** The substrate is what is
  depended on; the concrete is what depends. That is mechanically checkable
  from `Cargo.toml`, unlike any naming convention.
- **Detection ≠ authorization (principle 0, `agent_config.md`).** The glass is
  where that boundary physically lives: observations (outside) report data in;
  the gate (inside) emits decisions out. The two must not be coupled.
- **Generic over the wire representation.** The concrete (the wire
  representation, the version) is *data flowing through* the glass, not a
  parallel implementation. You do not make `host-graph-v1` and `host-graph-v2`.

## Decision

1. **The glass encloses the stable, generic, depended-on substrate.** Crabjar's
   own machinery (guard, host-core, the exec pipeline) is inside. The concrete —
   integrations, wire representations, version-specific behavior — is outside
   and is meant to be disposable.
2. **A good abstraction is one the outside can come and go through and the
   inside doesn't flinch.** Operational test: *can you swap the concrete without
   touching the abstraction?* If the answer is no, the concrete has welded
   through the glass (anti-pattern: `vm-bridge`).
3. **The prefix is ownership, not provenance.** `crabjar-*` = crabjar's own
   infrastructure (inside). No prefix = an integration with, or abstraction
   over, the outside world (outside). This is the naming rule that falls out of
   the glass — it is *not* a provenance/experimentality signal in either
   direction (tested and rejected against the dependency graph: `crabjar-guard`
   and `crabjar-host-core` are the two most load-bearing crates, so "prefix =
   experimental" is false; `agent-context` is proven but unprefixed, so
   "prefix once proven" is false).
4. **Agents face the right direction.**
   - *Inside (substrate) agents look OUT.* They define and consume the
     contract; their decisions flow outward as data through the glass. They
     never name a specific concrete from inside.
   - *Outside (integration/fleet) agents look IN.* They consume the contract
     and report concrete results inward as data. They never reach in and mutate
     the contract.
   - Shouting in the wrong direction = coupling across the glass: concrete↔
     concrete (the `vm-bridge` case), or an abstraction hardcoding a concrete.
     This is detection ≠ authorization made spatial.

## Options Considered

### Option 1: Status quo — no documented boundary

Keep the naming ad hoc; fix the footgun per-instance.

- **Offers**: no documentation cost
- **Rejected because**: the footgun recurs (17/25 crates mismatch), and the
  deeper failure — agents coupling across the glass — has no named rule to
  point at. `vm-bridge` is already an instance of it.

### Option 2: Rename all crates so directory == package name

Make `guard` the package name, `memory` the package name, etc.

- **Offers**: the `-p <dir>` form would just work
- **Rejected because**: churn across every `path = "..."` dependency, CI, and
  doc for no functional gain; and it discards the ownership signal the prefix
  carries. The generic names (`guard`, `memory`, `sandbox`) would also collide
  in the global crate namespace. The footgun is mechanical and has a cheaper
  fix (a Justfile resolver); the boundary is conceptual and needs a rule.

### Option 3: Document the glass + directional rule (chosen)

Pin the boundary as an ADR, add the orientation rule to the agent docs, and
keep the mechanical footgun fix (Justfile resolver) separate.

- **Offers**: a principled answer to both the naming question and the
  orientation question; new crates get named by ownership; agents have a rule
  for which way to shout; `vm-bridge` becomes identifiable as the anti-pattern
- **Costs**: "ownership" is a judgment call for genuinely ambiguous crates
  (see Ongoing concerns); the boundary is a convention, not mechanically
  enforced by `crabjar-architecture` (which enforces layer deps, not the glass)

## Consequences

### Positive consequences

- **The naming footgun gets a principled answer** and a mechanical fix (the
  Justfile directory→name resolver), decoupled from the conceptual rule.
- **New crates are named by ownership**, not ad hoc — `crabjar-*` for own
  infrastructure, descriptive name for an integration/abstraction over the
  outside.
- **Agents have an orientation rule**: which side of the glass, which way to
  shout. Fleet leaves report data in; the substrate verifies before it becomes
  state.
- **Detection ≠ authorization is made spatial.** The glass is where principle 0
  physically lives: the observation side reports in, the gate side decides out,
  and the two are not coupled.
- **`vm-bridge` is now nameable as the anti-pattern** (concrete welded through
  the glass), giving the next reviewer a term for the failure.

### Negative consequences (trade-offs)

- **"Ownership" is a judgment call** for crates that are crabjar's but named
  after a general concept (`agent-context`, `orchestrator`) — the prefix is
  optional there and consistency-within-a-subsystem is the tie-breaker.
- **The boundary is a convention**, not a CI gate. `crabjar-architecture`
  enforces the 8-layer dependency model, not the glass per se; a concrete
  welding through the glass is caught by review, not by a test.

### Ongoing concerns

- **`vm-bridge` / `axum-mux` needs a deliberate decision.** It is the one crate
  that breaks every naming theory (three-way mismatch: directory, package, and
  capability all disagree). Resolve it on its own terms — do not force it into
  the ownership rule.
- **Do the mechanical fix separately.** The Justfile resolver (directory→name
  at runtime) is policy-agnostic and should land regardless of how the naming
  rule is worded.
- **Revisit when the substrate changes.** If crabjar's own machinery is ever
  split out or an integration is promoted to core, the glass moves. Re-run the
  "can you swap the concrete without touching the abstraction?" test.

## References

- Conversation 2026-08-30 — the `guard`/`crabjar-guard` footgun → the glass
  question → the fleet/internal orientation rule.
- [[ADR-002]] — Herdr as execution substrate (the glass is the clutch
  boundary; the concrete is the flywheel side).
- [[ADR-003]] — Spatial habitat layer (clutter as memory; the same
  "representation downstream of state" discipline).
- `agent_config.md` principle 0 — detection ≠ authorization (the glass is this,
  made spatial).
- Crabjar design preference — "generic over the wire representation, not over
  versions."
- `axum-mux/Cargo.toml` (package `vm-bridge`, no lib target) — the standing
  anti-pattern example.
