---
id: ADR-003
title: Spatial Habitat Layer
status: Proposed
date: 2026-08-23
see_also: [[ADR-002]]
---

# ADR-003: Spatial Habitat Layer

## Status

**Proposed**. This ADR pins crabjar's spatial layer: crabjar maintains a
persistent *spatial habitat* — a map of computational state laid out over a
model of the user's lived environment — and fixes three constraints on it:

1. Home Assistant is the physical truth source; it does not own the virtual world.
2. Rendering is deliberately low-fidelity cartography, not a simulation.
3. Divergence between the physical environment and the model is exposed, never auto-corrected.

## Context

[[ADR-002]] fixed the vertical stack: Herdr is the flywheel/mechanism, crabjar
is the clutch, frontends are consumers of the clutch primitive. It named the
spreadsheet frontend ("cells as execution handles") but did not say what a
frontend *represents*. The 2026-08-23 conversation (clipping:
`Multiplexed Agent Spreadsheet.md`) supplies the missing thesis:

> **Crabjar should represent lived space, not simulate living space.**

The core idea is that **computational clutter is memory**. An agent that
finishes a task does not have to disappear — it can leave behind an artifact,
a pending decision, a failed attempt, a warning, a suspended runtime, an
unresolved action. Those things *occupy space* in a model of the user's
environment. A messy desk means unresolved state; a clean room means little
unresolved state; a pile getting bigger means something is not being resolved;
something disappearing means state was consumed or archived. The clutter
itself becomes a visualization of computational inertia, and the user remembers
"that's the thing I left on the workbench" instead of
`agent_7f92c → task_2841 → pending_guard_action_91`.

This is deliberately different from a digital twin in the established sense.
Existing spatial work (VR floor plans, HA 3D dashboards with live entities at
3D coordinates) maps **physical reality → clean digital representation**.
Crabjar proposes **physical reality → persistent computational habitat**,
where the habitat is allowed to become messy in the same way the person's real
environment does.

Forces at play:

- **The HA seam already exists.** `host/host-mqtt` implements MQTT + Home
  Assistant discovery (layer 4). HA models physical entities, groups them into
  areas, exposes their state, and provides local control; Matter/Thread extend
  it to low-power sensors. The physical truth layer already has a home in this
  workspace.
- **Detection ≠ authorization already forbids the failure mode.** Principle 0
  in `agent_config.md` says observer reports must not trigger execution. The
  habitat extends the same principle to *representation*: observation of
  physical change must not silently rewrite the spatial model.
- **The rendering temptation.** A realistic render invites the user to
  *perform* the environment — clean the virtual desk because it looks ugly,
  arrange objects because they look aesthetically wrong, manipulate objects
  because the simulation invites it. The user ends up optimizing the render
  instead of the state, and the representation becomes false. The
  representation must be downstream of state, not a stage where the user
  decorates state.

## Decision

1. **Crabjar owns a persistent spatial model.** The habitat is a first-class
   crabjar data structure: a model of the user's lived environment (areas,
   surfaces, positions — coarse geometry, not a scan) over which computational
   state is laid out. Entities in the model: agents (with Herdr's
   `working`/`blocked`/`idle` state), artifacts, pending guard actions,
   suspended runtimes, unresolved decisions. Their *presence* in the model is
   state, not decoration.
2. **Home Assistant is the physical truth source; it does not own the virtual
   world.** The asymmetry is pinned:
   - HA says: *"The desk is here. It's 22.4°C. The light is on. Someone is
     sitting here."*
   - Crabjar says: *"This is the desk. Three agents, a suspended job, and an
     unresolved decision are here."*

   HA feeds the sensor/actuator layer (via `host-mqtt`); crabjar feeds the
   spatial memory layer. HA entities map to model positions; the mapping is
   crabjar's, not HA's.
3. **Rendering is deliberately low-fidelity.** The perceptual surface (TUI
   today; Bevy/GPU later) renders the habitat as information-dense,
   low-resolution cartography — a persistent diagram of computational life,
   not a videogame apartment. Fidelity is a *constraint*, not an aesthetic: it
   prevents visual realism from becoming an invitation to perform the
   environment. The rule: **the representation is downstream of state.**
   Clutter appears because state is unresolved; it disappears because state
   was resolved, archived, or consumed — never because the user found it ugly.
4. **Divergence is exposed, never auto-corrected.** When the physical
   environment and the spatial model disagree (presence change, HA entity
   update, the user physically moved things), crabjar surfaces a discrepancy:
   *"Physical environment and spatial model diverged."* It does not propose or
   perform a sync. The human decides which representation is authoritative,
   and may deliberately let the two stay out of sync — the divergence itself
   is useful information ("my physical desk is clean but my computational
   desk still has six abandoned agents on it"). This is detection ≠
   authorization applied to representation: observe → expose discrepancy →
   don't silently couple → let the human decide.
5. **Bidirectional coupling crosses the clutch.** Any future path from the
   spatial model to a physical actuator (moving the virtual representation of
   an object → a desired physical action) passes through the guard gate like
   any other world-affecting action. The UI never drives HA directly.

## Options Considered

### Option 1: Status quo — no spatial layer

Keep crabjar a trust layer + terminal/agent harness with a flat TUI.

- **Offers**: no new surface to maintain
- **Rejected because**: the state crabjar already accumulates (pending guard
  actions, work items, agent states) has no navigable representation. The
  ADR-002 frontend slot stays empty, and accumulated state is addressable
  only by ID chains, not by memory.

### Option 2: Full digital twin — realistic 3D replica

LiDAR-scan the home, render a faithful 3D environment, place live entities in
it (the HA 3D-floorplan pattern).

- **Offers**: most intuitive spatial interface; existing prior art
- **Rejected because**: realism triggers game-like expectations and the urge
  to *perform* the environment — clean the virtual desk, arrange objects,
  manipulate for aesthetics. The user optimizes the render, not the state.
  The render becomes a stage and the state it represents becomes false.

### Option 3: Spatial habitat with low-fidelity cartography (chosen)

Model the environment abstractly (areas/surfaces/positions, not scan
geometry), lay computational state over it, render as an information-dense
diagram.

- **Offers**: clutter = memory (unresolved state is visible and addressable);
  the render cannot be mistaken for a world to decorate; cheap to build and
  maintain (no asset pipeline, no 3D fidelity budget); the desync principle
  keeps the human authoritative
- **Costs**: less immediately legible than a realistic room; requires a model
  of "where things are" that does not come from a scan (the user places
  entities, or HA areas provide coarse geometry)

### Option 4: HA owns the spatial model

Put the habitat inside Home Assistant (custom dashboard / 3D floorplan).

- **Offers**: HA already has areas, entities, and dashboards
- **Rejected because**: it puts crabjar's state inside another product's
  schema and lifecycle — the same error as ADR-002's rejected Option 2 (the
  clutch inside the flywheel). HA's model is physical; the habitat is
  computational. The owner of the authority must own the model of the
  authority's state.

## Consequences

### Positive consequences

- **Clutter becomes a metric.** A pile getting bigger = something isn't being
  resolved. Something disappearing = state was consumed or archived. The
  spatial model is a visualization of computational inertia with no dashboard
  to maintain.
- **The ADR-002 frontend slot gets a concrete shape.** The spreadsheet (cells
  as execution handles) and the habitat (positions as execution handles) are
  the same primitive: an addressable cell that consumes the clutch and grows
  no trust logic of its own.
- **HA integration gets a job.** `host-mqtt`'s discovery is currently a pipe;
  the habitat gives it a consumer (presence, area, and sensor state → model
  positions and coupling state).
- **The desync principle is consistent with the existing trust model.** It is
  detection ≠ authorization applied to representation — no new authority
  surface, no new gate logic.

### Negative consequences (trade-offs)

- **A new persistence surface.** The spatial model needs storage (positions,
  entity→position mapping, divergence records). Likely new tables on the
  existing SQLite substrate, but a new schema to version.
- **A second thing to keep honest.** The model can diverge from both the
  physical room and the computational state; the state-doc staleness tiers
  will need a habitat analogue.
- **The render is less legible to outsiders.** A diagram of computational
  life is not self-explanatory to someone who hasn't lived in it.

### Ongoing concerns

- **The "where" problem.** The model needs positions. Coarse geometry from HA
  areas is enough to start; LiDAR scans are a future input, not a requirement.
- **Presence-driven coupling.** When HA detects absence, the habitat can
  *suspend coupling* (agents keep running, state persists, spatial state
  persists) — but that is a clutch action and must be explicit, not a silent
  default.
- **The render must stay ugly.** Even beautiful 16-bit art becomes something
  you want to decorate. Revisit the fidelity constraint every time the
  perceptual surface changes (TUI → Bevy).
- **Next artifact:** a minimal habitat spike — a SQLite-backed position store
  + a TUI panel that renders pending guard actions and agent states as
  positioned entities. The ADR fixes the nouns and the boundary; the spike
  proves the seam.

## References

- Clipping: `Multiplexed Agent Spreadsheet.md` — the source conversation
  (2026-08-23): multiplexed spreadsheet, computational clutter, HA layering,
  anti-simulation rendering, intentional desync
- [[ADR-002]] — Herdr as execution substrate (flywheel/mechanism/clutch/
  frontend nouns)
- `host/host-mqtt/AGENTS.md` — MQTT + HA discovery (the physical truth seam)
- `agent_config.md` principle 0 — detection ≠ authorization
- `memory/src/state_docs/models.rs` — staleness tiers (the habitat needs an
  analogue)
