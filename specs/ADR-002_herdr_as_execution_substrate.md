---
id: ADR-002
title: Herdr as Execution Substrate
status: Proposed
date: 2026-08-23
see_also: [[ADR-001]]
---

# ADR-002: Herdr as Execution Substrate

## Status

**Proposed**. This ADR pins the architectural relationship between crabjar and
[Herdr](https://herdr.dev/): Herdr is the execution substrate (the mechanism
that keeps work in motion); crabjar is the trust layer (the clutch that decides
when that motion couples to the world). It also fixes the integration mode:
crabjar drives Herdr as a client/orchestrator, never as a plugin inside it.

## Context

Crabjar has historically been both the runtime *and* the trust layer. The
terminal/relay machinery is hand-rolled in-repo:

- `axum-mux/src/terminal_relay.rs` — axum-based terminal relay
- `src/vm_bridge/relay.rs` — VM bridge relay

Meanwhile crabjar's actual identity is the trust layer: `ExecutionGate`
(`guard/src/gate.rs`), scope isolation, fingerprint approvals, the concierge,
and the `doubt` block on every derived output. Pending actions persist to
`GuardDb` (`guard/src/guard_db.rs`) — a pending action is a decision sitting
in the clutch while the agent keeps running.

Two forces make the split necessary:

1. **Always-on, remote work.** The expensive state we want to keep warm —
   models resident, GPU context loaded, agents mid-task (e.g. the pesti
   inference substrate, where `model unloaded → model resident` and
   `CPU → GPU` transitions are the costly part) — must survive lid-close and
   reconnect from any keyboard. crabjar's relays do not provide this story.
2. **A mature runtime already exists.** Herdr (herdr.dev) is an always-on
   terminal/agent runtime: multi-agent, structured agent state
   (`working` / `blocked` / `idle`), a socket API, and Tailscale remote
   access. It is deliberately **trust-neutral** — it owns terminals and
   agents, but does not decide who is allowed to do what.

The codebase already has the seam: `TerminalBackend` in
`crates/terminal/src/backend.rs` is backend-agnostic (wezterm > zellij
auto-detection in `TerminalManager`), and `host/host-core/src/adapter.rs`
establishes the ProductAdapter precedent — new backends slot in without core
changes.

The decision needed: which side of the Herdr seam does the trust boundary
live on, and what is each component's job?

## Decision

1. **Herdr is the execution substrate.** It owns terminals, tabs, panes,
   agents, and their lifecycle, including remote access over Tailscale. It
   keeps the "flywheel" (warm models, running agents, resident state)
   spinning.
2. **Crabjar is the clutch.** It generates no power and steers nothing; it
   decides when the flywheel's momentum couples to the world. Every action
   that produces real-world effects — `agent start`, `pane run`,
   `agent prompt`, any tool invocation — passes through the guard gate.
   "Disengaged" means *not yet coupled*, not *dead*: agents keep running,
   state keeps accumulating, pending actions sit in `GuardDb`.
3. **Integration mode: client/orchestrator.** Crabjar stays its own binary
   and drives Herdr over the socket API. The guard gate wraps every
   outbound action. Crabjar is **not** a plugin inside Herdr.
4. **The trust boundary is pinned to the crabjar side of the seam.** Herdr
   remains trust-neutral. No trust/authorization logic is added to or
   delegated to Herdr. The authority the human owns must be the thing the
   human owns when they drive away.
5. **Implementation seam: `HerdrBackend`.** A third `TerminalBackend`
   implementation (alongside wezterm and zellij) following the
   ProductAdapter pattern. No core changes. Method mapping is near 1:1:
   `spawn` → workspace/tab create + agent start; `send_text` → pane
   send-text / agent send-keys; `read_output` → pane/agent read;
   `kill_session` → kill pane; `split_pane_h/v` → pane split.
   `SpawnResult.pane_id` aligns with Herdr's pane IDs (`w1:p2`).
6. **Naming for the stack** (used in docs and future ADRs):
   - **flywheel** — the expensive warm state (pesti models, GPU context, running agents)
   - **mechanism / substrate** — Herdr: keeps it spinning, survives disconnects
   - **clutch** — crabjar: engage/disengage, human in the loop
   - **frontend** — future surfaces (e.g. a spreadsheet whose cells are
     execution handles). Frontends are *consumers* of the clutch primitive;
     they never carry trust logic of their own.

## Options Considered

### Option 1: Status quo — keep hand-rolling the relay

Continue maintaining `terminal_relay.rs` / `vm_bridge/relay.rs` as crabjar's
runtime.

- **Offers**: zero new dependencies; full control
- **Rejected because**: it reinvents what Herdr already solves (persistence,
  reconnect, multi-agent state, Tailscale), and it keeps the mechanism
  tangled with the trust layer — the exact coupling this ADR exists to
  remove.

### Option 2: Crabjar as a Herdr plugin (rejected)

Embed crabjar inside Herdr's process as a plugin; the guard gate runs
in-Host of the substrate.

- **Offers**: tightest integration; no socket hop
- **Rejected because**: it puts the clutch inside the flywheel. The guard
  ends up in Herdr's lifecycle, sandbox, and release cycle — crabjar would
  be trusting the moving mass to control its own authority. The trust layer
  must survive independently of the substrate it gates.

### Option 3: Crabjar as client/orchestrator (chosen)

Crabjar stays its own binary, connects to a Herdr server (local socket or
tailnet `host:port`), and gates every action before it is sent.

- **Offers**: trust boundary stays on the crabjar side of the seam;
  crabjar's lifecycle is independent; the guard's existing primitives
  (ExecutionGate, GuardDb, concierge, doubt blocks) are reused unchanged;
  Herdr's structured agent state (`working`/`blocked`/`idle`) can drive
  the approval flow instead of screen-scraping
- **Costs**: a new external dependency and a network hop (see Consequences)

### Option 4: Keep crabjar's relays for local, use Herdr only for remote

- **Offers**: no local-path change
- **Rejected because**: two mechanisms for one job guarantees drift. The
  hand-rolled relays overlap Herdr's purpose; they need a deprecation path,
  not a permanent coexistence.

## Consequences

### Positive consequences

- **Crabjar stops maintaining the mechanism layer.** Persistence,
  reconnect, multi-agent, and remote access are Herdr's problem, not ours.
- **Agent state comes for free.** Herdr's `working`/`blocked`/`idle` states
  feed the guard's approval flow directly — no terminal-screen scraping.
- **The flywheel stays warm.** pesti-class runtimes can live on the GPU box
  over Tailscale; `model unloaded → model resident` and `CPU → GPU`
  transitions stop being paid on every demand.
- **Small, proven seam.** `HerdrBackend` implements an existing trait via
  the ProductAdapter pattern; no core changes, and the 8-layer dependency
  model is untouched (a host-layer crate depends on layers 0–4 only).
- **Frontends become cheap.** Anything that can execute can occupy a cell /
  pane / tab, and every one of them is gated by the same clutch primitive.

### Negative consequences (trade-offs)

- **New external dependency.** Crabjar's integration surface is now pinned
  to Herdr's socket API and release cadence. API drift is a real cost.
- **More failure modes in `is_available()`.** It becomes "is a Herdr server
  reachable (local socket or tailnet)?" rather than "is the binary on
  PATH?". `HerdrBackend` carries connection state (socket path or
  `host:port`) and must handle server-down and tailnet-down distinctly.
- **Hand-rolled relays become redundant.** `axum-mux/src/terminal_relay.rs`
  and `src/vm_bridge/relay.rs` need an explicit deprecation path; silent
  coexistence is worse than either option.
- **Remote approval UX.** The guard's a/r approval flow must work over the
  tailnet — latency and reconnect mid-approval are new edge cases.

### Ongoing concerns

- **Herdr API stability.** Re-validate the method mapping on each Herdr
  release; treat the socket API as a contract with an external party.
- **If Herdr ever ships its own trust/authorization layer**, revisit the
  boundary — but the rule holds regardless: authority the human owns stays
  on the crabjar side of the seam.
- **When a frontend arrives** (spreadsheet cells as execution handles),
  verify it consumes the clutch primitive and grows no trust logic of its
  own.
- **Next artifact:** a minimal `HerdrBackend` spike against a local Herdr
  server (create a workspace, start one agent, read its state). The ADR
  fixes the nouns and the boundary; the spike proves the seam.

## References

- Herdr docs: <https://herdr.dev/>, <https://herdr.dev/docs/agents/>, <https://herdr.dev/docs/agent-automation/>
- `crates/terminal/src/backend.rs` — `TerminalBackend` trait (the seam)
- `host/host-core/src/adapter.rs` — ProductAdapter precedent
- `guard/src/gate.rs`, `guard/src/guard_db.rs` — ExecutionGate and pending-action persistence ("disengaged ≠ dead")
- `axum-mux/src/terminal_relay.rs`, `src/vm_bridge/relay.rs` — hand-rolled relays slated for deprecation
- pesti `README.md` — flywheel state transitions (model residency, CPU↔GPU)
- Nygard, M. (2011). *Documenting Architecture Decisions*. https://www.infoq.com/articles/Architecture-Decision-Lang
- `specs/README.md` — ADR process and conventions for this project
