# Session handoff: crabjar — typed terminal stream (ADR-005)

Companion to `specs/ADR-005_typed_terminal_stream.md`. Read that first — it is
the spec; this file tells you what exists, what was rejected, and where to
start. The goal: a **typed, addressable terminal event stream** with
**type-safe copy-paste**, where asciinema v2 / WebSocket JSON / native JSONL
are swappable wire representations, and the `axum-mux`/`vm-bridge` relay is a
thin transport.

## What exists today (verified, not from docs)

Two half-built stubs of the same idea. Neither is a working baseline — this is
greenfield on top of them.

### 1. `crates/terminal/` (package `crabjar-terminal`) — ✅ compiles, stream model live
- `src/stream.rs` (394 LoC) — **the ADR-005 substrate**: `TerminalEvent`
  (Prompt/Command/Output/Raw, monotonic ids + per-event `at` timestamp),
  `SessionStream`, `Block` (+ `Block::receipt`), `Receipt`,
  `STREAM_VERSION = 2` (v2 added `at`; v1 records still parse — `at`
  defaults to the epoch).
- `src/session_record.rs` (188 LoC) — `SessionRecord`: versioned native
  JSONL (the faithful on-disk form), `to_jsonl`/`from_jsonl`/`save`/`load`,
  v1-migration test.
- `src/copy_paste.rs` (227 LoC) — **ADR-005 item 3**: `Selection`
  (event-range or block copy), `copy_event_range`/`copy_block`,
  `paste_jsonl` (faithful), `paste_asciinema` (lossy), `Selection::receipts`
  (reconstruct receipts from a selection alone).
- `src/verifiers.rs` (342 LoC) — **the deterministic task scorers**
  (README parity item): `exit_code`, `file_exists` (relative paths resolve
  against the receipt's cwd), `regex_match`, `json_path` (dot-path into the
  output's JSON). Pure functions of `(Receipt, expectation)` →
  `VerifierResult { verifier, passed, detail }`.
- `src/herdr_exec.rs` (160 LoC) — `HerdrBackend::run_command` → typed
  `Receipt` via the structured round-trip (see Remaining work #1).
- `src/recording.rs` (415 LoC) — `AsciinemaSerializer` (renamed from
  `AsciinemaRecorder`): asciinema v2 is now a **serializer of the stream**,
  not the model. Batch mode (`from_events`) and live mode (`event`, fed
  from `TerminalSession::send`/`read`). Lossy by design — documented.
- `src/lib.rs` (346 LoC) — `TerminalSession` + `TerminalManager`.
  `send`/`read` now append to the session's `SessionStream` (send →
  `Command`, read → `Raw` escape hatch) and feed the serializer when
  recording. New: `session_record()` accessor.
- `src/backend.rs` (95 LoC) — `TerminalBackend` trait: `spawn`, `send_text`,
  `read_output` (last N lines), `kill_session`, `split_pane_*`.
- `src/herdr.rs` (376 LoC) — the ADR-002 execution substrate backend
  (session mapping + trait impl). Structured execution moved to
  `herdr_exec.rs` (500 LoC gate).
- `src/wezterm.rs` (267), `src/zellij.rs` (248) — backend impls.
- `examples/herdr-spike.rs` — ADR-002 backend spike (still green).
- `examples/herdr-stream-spike.rs` — **ADR-005 verification**: 3 real
  commands → 3 Receipts → 3 blocks → JSONL round-trip → copy-paste
  (block copy, both paste targets, receipt reconstruction) → all four
  verifiers run against the live receipts. Run:
  `cargo run -p crabjar-terminal --example herdr-stream-spike` (needs a
  live herdr server).
- `Cargo.toml` description already says "asciinema recording" — the idea is
  half-present in the tree.

### 2. `axum-mux/` (package `vm-bridge`) — ✅ builds natively, relay drops input
- **It is NOT WASM-only.** Both `axum-mux/AGENTS.md` and the dependent crates'
  AGENTS.md say "vm-bridge has no lib target (WASM-only) — won't compile on
  native." **That is false.** It built natively in 5.1s (`cargo build -p
  vm-bridge`), no CUDA, no wasm target. It is a **binary-only native crate**
  (tokio + axum + TCP/websocket). The "WASM-only" annotation is stale — fix it
  regardless of what you decide.
- `src/main.rs` (93) — supervisor/worker re-exec model: one OS process per VM
  (a segfault in one VM's proxy is contained), exponential-backoff restart.
  Modes: default (supervisor), `--worker <name>`, `--terminal-relay <port>`.
- `src/proxy.rs` (172) — byte-transparent SPICE/VNC websocket relay (browser
  does the protocol decode; the proxy just forwards bytes). **This concrete
  stays raw binary — do not force it into the typed stream** (ADR-005
  Decision 5).
- `src/terminal_relay.rs` (293) — multi-client WebSocket relay, JSON control
  frames + binary I/O. **Line 253: "Terminal session send_text integration is
  pending — currently drops input silently."** This is the stub that becomes
  the thin `TerminalEvent` transport.
- `src/manifest.rs` (149) — `Manifest`/`Vm` TOML parsing, 6 passing tests.
- `manifest.toml` — placeholder `bind_addr = "100.x.x.x"`, two sample VMs.
- 707 LoC / 4 files total.

### 3. Compilation status
- `cargo build -p vm-bridge` — ✅ passes (5.1s, native).
- `just test-crate guard` — ✅ 10 pass (the resolver from the previous
  session works; use `just test-crate <dir>` for narrow-scope tests — see
  Environment).

## What was NOT done (rejected)

- **Keep the relay byte-transparent (status quo)**: Rejected because it can't
  deliver addressable text or type-safe copy-paste — the actual goal — and
  leaves both stubs dead.
- **Make `vm-bridge` a lib + bin, embed in host-screen/apps-teams**: Rejected
  because nothing embeds it today (`grep vm_bridge` in host-screen and
  apps-teams `src/` returns nothing). Adding a lib surface nobody calls is
  premature. The typed stream is the reason to add a lib, and it lives in
  `crabjar-terminal`, not the relay.
- **Adopt asciinema v2 as the source-of-truth model**: Rejected because its
  `i`/`o` event model has no block structure, no exit codes, no ids. It cannot
  express addressable text or receipts without a parallel layer — which just
  re-creates the typed stream on top. Build the model first; asciinema becomes
  a *serializer*.

### ⚠️ Critical: three dead lib-deps emit a warning on every build
`vm-bridge` is binary-only (no `lib.rs`), yet three crates declare
`vm-bridge = { path = "axum-mux" }` as a *library* dependency:
- root `Cargo.toml:234`
- `host/host-screen/Cargo.toml:20`
- `apps/teams/Cargo.toml:19`

Cargo warns `ignoring invalid dependency vm-bridge which is missing a lib
target` on **every** build. **None of the three actually use it** —
`grep vm_bridge` in their `src/` returns nothing. The deps are dead; they only
exist to emit the warning. Drop all three (and the root workspace dep) — this
is independent of the ADR-005 design and should happen regardless.

### ⚠️ Critical: the "WASM-only" doc claim is wrong
`axum-mux/AGENTS.md` and the dependent crates' AGENTS.md all say vm-bridge is
WASM-only and "won't compile on native." It is a native binary; it built in
5.1s. Correct the annotation so the next agent isn't misled.

## Remaining work (in priority order)

### ~~1. Prove segmentation on ONE backend — herdr~~ ✅ DONE (2026-08-30)
Proven via a different route than predicted: herdr 0.8.2 does not emit
per-command structured events, so the backend drives its own round-trip.
`HerdrBackend::run_command` (`crates/terminal/src/herdr_exec.rs`):
`pane run` a marker-wrapped line → `pane wait-output --regex` the
exit-code sentinel → `pane read --source recent` sliced between markers
(retry loop for buffer flush). `pane get` supplies `cwd`. Verified live:
`cargo run -p crabjar-terminal --example herdr-stream-spike` → 3 real
commands (multi-line, subshell exit 7, pwd) → 3 Receipts → 3 blocks →
JSONL round-trip, `success: true`.
**Pitfalls found (baked into the code):** `--match` fires on the pane's
echo of the submitted line (use `--regex '^<sentinel>[0-9]'`); `pane get`
nests the pane under `.pane` (the old top-level reads always returned
None); read after wait needs a retry (wait can match before the read
buffer flushes); `exit N` in the pane's interactive shell kills the pane
(subshell it in tests).

### 2. Define the `TerminalEvent` model + block addressing ✅ DONE (2026-08-30)
`crates/terminal/src/stream.rs` (430 LoC): `TerminalEvent` (Prompt/Command/
Output/Raw, monotonic ids assigned by `SessionStream::push`), `Block`
grouping, `Receipt`, `SessionRecord` (versioned native JSONL,
`STREAM_VERSION = 1`). 5 unit tests. Landed in `crabjar-terminal`, not a
new crate (see ADR-005 ongoing concerns).

### 3. Type-safe copy-paste → receipts ✅ DONE (2026-08-30)
`crates/terminal/src/copy_paste.rs`: copy = `copy_event_range(stream,
first_id, last_id)` or `copy_block(stream, &block)` → a self-contained
`Selection`; paste = `Selection::paste_jsonl` (faithful sub-record) or
`paste_asciinema` (lossy projection); `Selection::receipts` reconstructs
`Receipt`s from the selection alone. The CodeWhale-style verifiers landed
in `crates/terminal/src/verifiers.rs` — `exit_code`, `file_exists`,
`regex_match`, `json_path` as pure functions of `(Receipt, expectation)`.
Verified live in the extended herdr-stream-spike (steps 6–7): block copy
by address, both paste targets round-trip, and all four verifiers run
against real receipts (including a negative control that must fail).

### 4. Make the recorder a serializer, not the model ✅ DONE (2026-08-30)
`AsciinemaRecorder` → `AsciinemaSerializer` (`recording.rs`): batch mode
(`from_events`) projects a whole stream/selection; live mode (`event`) is
fed from `TerminalSession::send` (→ `i` events) and `read` (→ `o` events).
`TerminalSession` now carries a `SessionStream` — every `send`/`read`
appends (read lands in `Raw`, the escape hatch: scrollback is unsegmented)
— and `session_record()` exposes the typed stream as a `SessionRecord`.
`STREAM_VERSION` bumped 1 → 2 (per-event `at` timestamp, needed for
asciinema's relative times); v1 records still parse (`at` defaults to the
epoch). Asciinema v2 stays a **lossy** projection — documented in the
module.

### 5. Make the relay a thin `TerminalEvent` transport ✅ DONE (2026-08-30)
`axum-mux/src/terminal_relay.rs` (436 LoC) now forwards `TerminalEvent`
frames. Binary I/O frames become `Raw` events (the escape hatch) fanned out
to all joined clients; the relay stamps `at`, never mints ids (the receiving
session re-stamps `id`). `publish_event()` is the producer's entry point;
`start()` returns the bound address plus the shared state handle.
`ControlMessage` tags are snake_case + `deny_unknown_fields`, so a text
frame is unambiguously a control message or a `TerminalEvent`. The per-client
loop `select!`s on the broadcast receiver — idle clients receive published
events (the old `try_recv`-after-inbound-traffic pattern dropped them).
Live round-trip tests (tokio-tungstenite, 5 tests) live in
`axum-mux/src/terminal_relay_tests.rs` (split off for the 500 LoC gate).
`proxy.rs` (SPICE/VNC) untouched — stays raw binary (ADR-005 Decision 5).
`crabjar-terminal` registered in the architecture layer model (layer 3).

### ~~6. Drop the three dead lib-deps + fix the WASM doc line~~ ✅ DONE (2026-08-30)
Deps dropped (root, host-screen, apps/teams), "WASM-only" annotations
corrected in the three AGENTS.md files and ADR-004 (counterexample now
past tense). `cargo check --workspace` is warning-free.

### 7. (optional, only if touching it) Rename `axum-mux`/`vm-bridge`
Per ADR-004 this is a concrete (integration, outside the glass) — no
`crabjar-` prefix is correct, but the name should describe the capability
(a terminal/display relay), not the framework. Rename only if you're touching
it anyway; it's churn across the `path = "..."` refs.

## Key implementation decisions

- **Typed stream is the substrate (inside the glass); wire formats are
  outside.** asciinema v2 / WebSocket JSON / native JSONL are serializers,
  never the source of truth (ADR-004).
- **`Raw` is the escape hatch.** When segmentation can't cleanly split
  command/output, bytes land in `Raw` rather than corrupting a boundary. A
  session is never "unrecordable" — worst case it is all `Raw`.
- **SPICE/VNC stays raw binary.** That is a different concrete — genuinely
  byte-transparent, no protocol decoding. Do not force it into the typed
  stream (ADR-005 Decision 5).
- **Prove on herdr first.** It's ADR-002's substrate and the most likely to
  already emit structured events. Generalize only after it's green.
- **Receipts are the consumer.** Design the `Output`/block shape so a command
  yields `{ command, output, exit_code, duration, cwd }` — that's what the
  CodeWhale verifiers need.

## What not to do

- **Don't make `vm-bridge` a lib to silence the warning.** The warning is
  caused by three dead deps; drop the deps (item 6), don't add a lib surface
  nobody calls.
- **Don't force SPICE/VNC into the typed stream.** It's a legitimately
  byte-transparent concrete.
- **Don't adopt asciinema v2 as the model.** It can't express blocks/exit
  codes/ids. It's a serializer.
- **Don't generalize segmentation to wezterm/zellij before herdr is green.**
  One backend, proven, then the rest.
- **Don't rename `axum-mux`/`vm-bridge` as a side effect.** It's a separate
  decision (item 7), churn across path refs.

## Environment

- **Narrow-scope tests:** `just test-crate <dir>` / `just check-crate <dir>` /
  `just clippy-crate <dir>` — pass a crate *directory* (e.g. `guard`,
  `crates/terminal`, `axum-mux`). These resolve the package name at runtime
  (added last session, commit `144056b`). Bare `cargo test -p guard` fails
  (package is `crabjar-guard`).
- **`just` is at `~/.cargo/bin/just`** but may not be on PATH in a fresh
  session — `export PATH="$HOME/.cargo/bin:$PATH"` first.
- **500 LoC rule** is a CI gate. `terminal_relay.rs` is 293 — room to grow,
  but if the `TerminalEvent` model + block logic lands in `crabjar-terminal`,
  watch `lib.rs` (346) and `recording.rs` (415); split by concern if they
  approach 500.
- **Git remote is SSH-only** (`git@github.com:CromboJambo/crabjar.git`) — never
  switch to HTTPS; if push fails, leave the commit local and tell the user to
  push from their terminal.
- **CI has no `just`** — any CI job must inline the bash, not call `just`.
- **Workspace package names ≠ directory names** for 17/25 crates (ADR-004: the
  glass). Resolve via `just test-crate <dir>`, not by guessing.

## Files to review (with purpose)

- `specs/ADR-005_typed_terminal_stream.md` — the spec; the source of truth for
  the design. Read first.
- `crates/terminal/src/herdr.rs` — the first segmentation target; check whether
  it already emits structured session events (decides the spike's shape).
- `crates/terminal/src/recording.rs` — the stub that becomes a serializer;
  note the unfed `record_input`/`record_output` and the lossy asciinema v2
  model.
- `axum-mux/src/terminal_relay.rs:253` — the "drops input silently" stub that
  becomes the thin transport.
- `axum-mux/AGENTS.md` — the stale "WASM-only" claim to correct (item 6).
