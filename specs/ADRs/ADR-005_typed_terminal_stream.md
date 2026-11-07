---
id: ADR-005
title: Typed Terminal Stream as Session Substrate
status: Proposed
date: 2026-08-30
see_also: [[ADR-004]], [[ADR-002]]
---

# ADR-005: Typed Terminal Stream as Session Substrate

## Status

**Accepted** (2026-08-30). The segmentation spike (Remaining work #1 in the
companion handoff) proved out on herdr: `HerdrBackend::run_command`
(`crates/terminal/src/herdr_exec.rs`) returns a typed `Receipt`
(`{ command, output, exit_code, duration, cwd }`) for real commands via the
structured round-trip — no PTY scraping. The `TerminalEvent` model, block
grouping, and native JSONL form live in `crates/terminal/src/stream.rs`;
copy-paste (`copy_paste.rs`), the four verifiers (`verifiers.rs`), and the
asciinema v2 serializer (`recording.rs`) are live;
`crates/terminal/examples/herdr-stream-spike.rs` is the verification.
Remaining: item 5 (the `axum-mux` relay as thin `TerminalEvent` transport) —
likely its own session.

## Context

The trigger was deciding what `axum-mux`/`vm-bridge` actually is. Reading the
code (not the docs) showed two half-built halves of the same idea, both stubs:

- `crates/terminal/src/recording.rs` — `AsciinemaRecorder`, asciinema v2
  (JSONL: header + `[time, "i"|"o", data]`). **Never fed.** In
  `crates/terminal/src/lib.rs`, `record()` starts the recorder and `stop()`
  closes it, but `send()` and `read()` never call `record_input`/
  `record_output`. It writes a header and an empty body.
- `axum-mux/src/terminal_relay.rs` — multi-client WebSocket relay (JSON
  control frames + binary I/O). Line 253: *"Terminal session send_text
  integration is pending — currently drops input silently."*

The user reframed the relay: **not** a byte-transparent SPICE/VNC proxy, but an
**asciinema stream / record with addressable text and type-safe copy-paste.**
That reframing dissolves the earlier "make it lib vs drop the dead deps vs
rename" question — the answer is that the typed event stream is the substrate,
and everything else is a wire representation.

This is [[ADR-004]] (the glass) applied directly: the typed stream is the
stable, depended-on substrate (inside); asciinema v2 / WebSocket JSON / native
JSONL are concrete wire representations (outside, disposable). It is also
[[ADR-002]]'s "cells as execution handles" applied to a terminal session: a
recorded session becomes a notebook of addressable, replayable blocks.

Forces at play:

- **Agents stop scraping output.** The CodeWhale parity item already adopted
  (`crabjar/README.md`: "deterministic task scorers — `exit_code` /
  `file_exists` / `regex_match` / `json_path` verifiers with typed receipts")
  needs a *typed* input shape. Today a verifier would regex over a raw log;
  with a typed stream it runs against a receipt
  (`{ command, output, exit_code, duration, cwd }`).
- **The recorder is dead code until fed.** The asciinema path exists but is
  empty; the relay drops input. Neither is a working baseline to break — this
  is greenfield on top of two stubs.
- **Segmentation is the real engineering cost.** PTY bytes → events (where
  does a command end and its output begin, across wezterm/zellij/herdr, with
  wrapped lines and interleaved output) is the hard part. Everything else is
  plumbing.

## Decision

1. **The typed event stream is the substrate (inside the glass).** A
   `TerminalEvent` type — append-only, each event carrying a monotonic id — is
   the source of truth for a recorded session. Wire formats are serializers of
   the stream, not the model.
2. **Event vocabulary (initial):**
   ```
   Prompt  { id, cwd? }
   Command { id, text, started_at }
   Output  { id, data, exit_code? }
   Raw     { id, data }        // fallback when segmentation fails
   ```
   `Raw` is the escape hatch: when prompt detection can't cleanly segment, the
   bytes land in `Raw` rather than corrupting a `Command`/`Output` boundary.
   A session is never "unrecordable" — worst case it is all `Raw`.
3. **Addressable text = two levels.** Events have monotonic ids (same shape as
   the guard's append-only event store). **Blocks** group events into
   prompt→command→output→next-prompt units. A block is the natural copy-paste
   unit and the addressable cell.
4. **Type-safe copy-paste.** Copy = select an event/block range. Paste =
   serialize to a typed target. The payoff: a command run in a session becomes
   a **receipt** (`{ command, output, exit_code, duration, cwd }`) — the input
   shape the CodeWhale-style verifiers consume.
5. **Wire representations are swappable (outside the glass):**
   - asciinema v2 — the existing `AsciinemaRecorder` becomes a *serializer* of
     the event stream, not the model.
   - WebSocket JSON — `terminal_relay.rs` forwards `TerminalEvent` frames, not
     raw PTY bytes.
   - native JSONL — crabjar's own recording format (the source of truth on
     disk).
   - **SPICE/VNC display relay stays raw binary.** That is a different
     concrete — genuinely byte-transparent, no protocol decoding. Do not force
     it into the typed stream.
6. **The relay becomes a thin transport.** `axum-mux`/`vm-bridge` forwards
   `TerminalEvent` frames between clients and the session; it owns no session
   state. The event model + block addressing + typed extraction live in
   `crabjar-terminal` (or a small new crate if the stream is decoupled from the
   multiplexer backends).

## Options Considered

### Option 1: Keep the relay byte-transparent (status quo)

Leave `vm-bridge` a raw SPICE/VNC + terminal byte relay.

- **Offers**: no segmentation work; the proxy is trivially correct
- **Rejected because**: it can't deliver addressable text or type-safe
  copy-paste — the user's actual goal. It also leaves both stubs (recorder
  unfed, relay dropping input) dead.

### Option 2: Make `vm-bridge` a lib + bin, embed it in host-screen/apps-teams

Add a `lib.rs`, keep `main.rs` thin, let the three dead deps become valid.

- **Offers**: kills the `missing a lib target` warnings; enables embedding
- **Rejected because**: nothing embeds it today (`grep vm_bridge` in
  host-screen and apps-teams `src/` returns nothing). Adding a lib surface
  nobody calls is premature. The dead deps get dropped regardless (see
  handoff). The typed stream is the reason to add a lib, and it lives in
  `crabjar-terminal`, not the relay.

### Option 3: Typed terminal stream as substrate (chosen)

The event stream is the model; asciinema/WebSocket/JSONL are serializers; the
relay is a thin transport; the three dead lib-deps are dropped.

- **Offers**: addressable text + type-safe copy-paste; agents get receipts
  instead of regex-scraped logs; the two stubs become one coherent design;
  glass-clean (stream inside, wire formats outside)
- **Costs**: PTY→event segmentation is real engineering; asciinema v2 is a
  *lossy* projection of the typed stream (its `i`/`o` events don't carry
  block ids or exit codes), so round-tripping through asciinema loses the
  addressable structure

### Option 4: Adopt an existing terminal-record format as the model

Use asciinema v2 (or a VTE-based format) as the source of truth rather than a
new typed stream.

- **Offers**: no new format to design; broad tooling
- **Rejected because**: asciinema v2's event model is raw `i`/`o` bytes with a
  timestamp — no block structure, no exit codes, no ids. It cannot express
  addressable text or receipts without a parallel layer, which just re-creates
  the typed stream on top. Build the model first; asciinema becomes a
  serializer.

## Consequences

### Positive consequences

- **Agents get receipts.** A recorded command is `{ command, output,
  exit_code, duration, cwd }` — the exact input the CodeWhale-style verifiers
  need. `exit_code` scoring stops being a regex over a log.
- **The two stubs become one design.** The unfed recorder and the input-dropping
  relay converge on a single `TerminalEvent` stream; both get a job.
- **A recorded session is a notebook.** Blocks are addressable cells
  (ADR-002) the spreadsheet/habitat frontend (ADR-003) can display and the
  agent can point at.
- **Wire formats are swappable.** asciinema v2, WebSocket JSON, and native
  JSONL are projections; none is the source of truth, so none can weld through
  the glass.

### Negative consequences (trade-offs)

- **Segmentation is the cost.** PTY bytes → events is the hard part; it must
  be proven on one backend before generalizing.
- **Asciinema v2 is lossy.** Its `i`/`o` model drops block ids and exit codes;
  a session round-tripped through asciinema loses its addressable structure.
  The native JSONL is the faithful form.
- **A new format to version.** `TerminalEvent` needs a version field and a
  migration path, like the state-docs schema.

### Ongoing concerns

- **Segmentation per backend.** Proven on **herdr** (ADR-002's execution
  substrate) — but via a different route than the spike predicted. Herdr 0.8.2
  does *not* emit per-command structured session events; instead the backend
  drives its own round-trip: `pane run` a marker-wrapped line,
  `pane wait-output --regex` the exit-code sentinel (a plain `--match` fires
  on the pane's echo of the submitted line, before the command runs), then
  `pane read --source recent` sliced between markers (with a retry loop — the
  wait can match before the read buffer flushes). `pane get` supplies `cwd`.
  wezterm's scrollback API is still the second tractable route; pattern-based
  prompt matching (pexpect-style) remains the fragile fallback.
- **Where the stream lives.** Decided at the spike: `crabjar-terminal`
  (`src/stream.rs`), with the herdr execution path in `src/herdr_exec.rs`.
  No new crate — the stream is small and the backends are the only
  producers.
- **The `vm-bridge`/`axum-mux` name.** Per ADR-004 this is a concrete
  (integration, outside the glass) — no `crabjar-` prefix is correct, but the
  name should describe the capability (a terminal/display relay), not the
  framework. Rename only if touching it anyway.

## References

- Conversation 2026-08-30 — the `vm-bridge`/`axum-mux` decision → the
  "asciinema stream with addressable text and type-safe copy-paste" reframe.
- [[ADR-004]] — The glass (the typed stream is the substrate; wire formats are
  outside).
- [[ADR-002]] — Herdr as execution substrate (cells as execution handles;
  herdr is the first backend to prove segmentation on).
- `crates/terminal/src/recording.rs` — `AsciinemaRecorder` (the stub that
  becomes a serializer).
- `crates/terminal/src/lib.rs` — `TerminalSession::record`/`send`/`read`
  (the recorder is never fed).
- `axum-mux/src/terminal_relay.rs:253` — "currently drops input silently" (the
  stub that becomes a thin transport).
- `crabjar/README.md` — CodeWhale parity: deterministic task scorers with typed
  receipts (the consumer of the receipts this ADR produces).
- `axum-mux/Cargo.toml` (package `vm-bridge`, no lib target) — the standing
  ADR-004 anti-pattern example.
