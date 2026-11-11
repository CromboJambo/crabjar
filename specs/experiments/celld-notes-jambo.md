# Celld Early-User Notes — jambo (node 1)

**Date**: 2026-11-10
**celld version**: 0.6.3
**Host**: jambo (x86_64, Artix Linux, dinit)

## Installation

```bash
# Binary from GitHub releases
curl -fsSL https://github.com/denoland/celld/releases/download/v0.6.3/celld-x86_64-unknown-linux-gnu.gz | gunzip > ~/.local/bin/celld && chmod +x ~/.local/bin/celld

# esbuild is REQUIRED (celld bundles workers with it)
curl -fsSL "https://registry.npmjs.org/esbuild/-/esbuild-0.28.2.tgz" -o /tmp/esbuild.tgz && \
  tar xzf /tmp/esbuild.tgz -C /tmp && cp /tmp/package/bin/esbuild ~/.local/bin/esbuild && chmod +x ~/.local/bin/esbuild
```

**Gotcha**: `celld dev` fails with "esbuild not found" if esbuild isn't on PATH. No clear error message pointing to the fix — just "install it and retry."

## Dev Mode (single node, local SQLite)

```bash
cd project/ && celld dev --host 127.0.0.1 --port 8787
```

- Uses `objects.sqlite3` as local object store (replaces S3 bucket)
- Per-cell SQLite databases in `.celld/dev/runtime/<Class>:<hash>/db.sqlite`
- LTX segments in `.celld/dev/runtime/<Class>:<hash>/ltx/e<N>/`
- Works with standard Wrangler config (`wrangler.jsonc`)

## Counter Example Results

```
GET /?name=test1 → {"n":1,...}
GET /?name=test1 → {"n":2,...}  (state persisted within session)
GET /?name=other → {"n":1,...}  (separate cell, separate state)
```

✅ Durable Object isolation works correctly
✅ State persists across requests within a session
✅ Each unique key gets its own cell with independent state

## Durability Finding ✅ Characterized

**Graceful shutdown (SIGTERM) PRESERVES cell state:**
- `dinitctl restart` → counter continued from 3 to 4 ✅
- Process manager sends SIGTERM → celld flushes LTX segments and closes SQLite cleanly

**Ungraceful kill (SIGKILL) LOSES cell state:**
- `objects.sqlite3` (metadata, bundled code) survives
- Cell directories remain but `db.sqlite` is gone — only empty `ltx/` dir
- Counter values lost on restart after kill

**Contract**: Always manage celld via a process manager that sends SIGTERM (dinit/systemd). Never SIGKILL unless you accept losing in-flight cell state. In fleet mode with S3, uploaded LTX segments provide durability even against SIGKILL.

## Running as a dinit Service

```ini
# ~/.config/dinit.d/celld-dev-counter
type = process
command = /home/crombo/.local/bin/celld-dev-counter.sh
```

Wrapper script sets PATH (for esbuild) and cd's to project dir. `dinitctl start/stop/restart celld-dev-counter` manages lifecycle with proper signal handling.

## State Architecture

```
.celld/dev/
├── objects.sqlite3          # Local object store (S3 replacement in dev)
│   ├── objects table        # Key-value: wake/format.json, deploy/counter/.../index.js
│   └── store_sequence table
└── runtime/
    └── Counter:<sha256>     # One dir per cell (class + id hash)
        ├── db.sqlite        # Cell's durable state (KV, etc.)
        └── ltx/e1/          # LTX replication segments (epoch 1)
            └── N.ltx
```

## Key Insight: Agent Sessions as Durable Objects

The `opencode` and `sandbox` examples reveal the pattern crabjar needs:

**Pattern:**
- Each agent session = one Durable Object instance (one cell)
- DO's built-in SQLite (`this.ctx.storage`) = persistent session state
- Session survives restarts + node failures (LTX replication in fleet mode)
- No external database — the DO IS the database

**opencode example** (`examples/opencode/`):
```js
const opencode = await OpenCodeWorkerd.create({
  storage: this.ctx.storage,  // ← DO's SQLite as agent persistence
});
const session = await opencode.session.create();
await opencode.session.prompt({ sessionID: session.id, text: PROMPT });
await opencode.session.wait({ sessionID: session.id });  // blocks until done
```

**sandbox example** (`examples/sandbox/`):
- Container-backed DO (Docker image with control server on :3000)
- DO = brain (tracks state), container = body (executes work)
- `/exec`, `/write`, `/serve`, `/processes` endpoints
- If node fails, DO state tells it what was running → restart on another node

**crabjar mapping:**
| crabjar today | with celld |
|---|---|
| Ephemeral agent sessions | Durable Object cells |
| Ad-hoc file state | DO SQLite storage |
| No failover | LTX replication + cell migration |
| Rust orchestration logic | Stays in Rust — celld is the state layer |

**This is the missing piece.** crabjar keeps its Rust orchestration (guard, trust, JEV) but offloads session persistence and failover to celld cells. The "early user taking notes" strategy means running real agent workloads through this pattern and documenting what works/breaks.

## Next Steps

1. [x] Install celld + esbuild on jambo
2. [x] Run counter example, verify state works
3. [x] Characterize durability (SIGTERM=persist, SIGKILL=lose)
4. [x] Run as dinit service with proper signal handling
5. [x] Map agent-session-as-DO pattern from examples
6. [x] **Deploy ftw3 as node 2** — celld + esbuild installed, counter running as dinit service, persistence verified
7. [x] **Build minimal Rust bridge** — `specs/experiments/celld_bridge_minimal.rs` proves:
   - Rust → HTTP → celld cell works (no celld deps, just curl + JSON)
   - State persists across service restart (counter 7→8, no reset)
   - Session-lifecycle contract validated: crabjar needs only HTTP + JSON
8. [x] **Authority failure test** — recovery does NOT implicitly authorize pending actions ✅ (`specs/experiments/celld_auth_failure_test.rs`)
9. [x] **GuardDb ↔ celld consistency boundary** — design spec + `SessionMirror` trait + HTTP implementation ✅
10. [ ] Test ensemble failover (kill node, verify cell migration) — needs fleet mode (MinIO)
11. [x] Run real agent workloads through cells — session-store worker + `crabjar cell` CLI + checkpointed workload demo ✅
12. [ ] **MinIO for fleet mode** — PARKED. Circle back when we need multi-node failover testing.

### Item #11: Real Agent Workloads Through Cells (2026-11-11) ✅ IMPLEMENTED

**What was built:**

1. **`session-store` worker** (`~/.local/share/celld-test/session-store/`, port 8789):
   - Per-session KV store with metadata index
   - API: `POST ?key=***` (create, 409 if exists), `GET ?key=***` (fetch, 404 if missing), `PUT ?key=***` (update), `?list=1` (enumerate)
   - One celld cell per agent session — state in built-in SQLite survives restarts

2. **Rust integration** (`src/celld_session.rs`, 219 lines):
   - `SessionClient` — HTTP client for the session-store worker, configured via `CELLD_SESSION_URL` (default `http://127.0.0.1:8789`)
   - `CheckpointedWorkload` + `WorkloadStep` + `StepStatus` — serializable workload model with checkpoint/resume semantics
   - Public API: `put()`, `get()`, `list()`, `delete()`

3. **CLI subcommand** (`crabjar cell ...`):
   - `cell put <session> <key> '<json>' [--type <t>]` — store value
   - `cell get <session> <key>` — retrieve value
   - `cell list <session>` — enumerate entries
   - `cell delete <session> <key>` — remove entry
   - `cell workload create <session> <name> <step1> [step2] ...` — create checkpointed workload
   - `cell workload status <session> <name>` — show progress
   - `cell workload step <session> <name> [--result <str>]` — execute next step (checkpoint after each)
   - `cell workload resume <session> <name>` — resume from last checkpoint

**Demo verified:**
```
create 4-step workload → complete 2 steps → (process dies) → resume → completes remaining 2 steps from checkpoint
```

**Key finding:** Agent lifecycle is now decoupled from machine lifecycle. The celld worker holds the state; the crabjar process is disposable. If the agent crashes or the machine reboots, a new process can `cell workload resume` and continue exactly where it left off.

**Celld early-user findings (item #11):**
- `storage.get()` returns `undefined` for missing keys, not `null` — must use nullish checks (`== null`) in worker code
- No `list({prefix})` on storage API — maintain an explicit index key for enumeration
- Dev mode SQLite persists across SIGTERM restarts (confirmed: data survived worker restart during testing)
- Worker runs fine as a simple background process; dinit service file written but dinit doesn't hot-reload service definitions in this environment

### Authority Failure Test (2026-11-10) ✅ PASSED

**Question:** Does node recovery implicitly authorize pending actions?

**Setup:** `approval-queue` worker on port 8788 (dinit service `celld-dev-approval`). Stores proposed actions with status: pending → approved/rejected. Only explicit PUT changes status.

**Test:** Stage action as pending → stop service (simulate crash) → verify unreachable → restart → GET action.

**Result:** Post-recovery: status=**pending**, decided_at=**null**. No implicit commit. Authorization flow works post-recovery.

**Implication:** The "migration ≠ authorization" boundary holds at the celld layer. crabjar's trust model survives infrastructure changes.

### GuardDb ↔ celld Consistency Boundary (2026-11-10) ✅ IMPLEMENTED

**Design:** `specs/experiments/guarddb-celld-consistency.md` — GuardDb is local authority, celld is durable mirror. Writes are fire-and-forget with retry; reads prefer local; reconciliation converges idempotently by action_id.

**Implementation:**
- `crates/guard/src/mirror.rs` — `SessionMirror` trait + `MirroredAction` struct + `MockMirror` for testing (286 lines)
- `specs/experiments/celld_session_mirror.rs` — `HttpSessionMirror` implementation talking to approval-queue worker via curl

**Integration test results (all pass):**
1. ✅ Mirror availability check
2. ✅ Stage pending action → verified in celld
3. ✅ Read back from celld (status=pending)
4. ✅ Update status to executed → celld shows approved
5. ✅ Verify decided_by recorded
6. ✅ Re-mirror of decided action is a no-op (no overwrite — fixed 2026-11-10)

**Overwrite fix:** `mirror_pending` now checks existing action status before POSTing. Approved/rejected actions are never re-staged; pending actions are idempotent. Terminal decisions in the mirror survive reconciliation attempts.

### ftw3 Deployment Notes

- `~/.local/bin/celld` 0.6.3, `~/.local/bin/esbuild` 0.28.2 (native binary from `@esbuild/linux-x64`, NOT the node wrapper)
- **Gotcha**: npm tarball's `bin/esbuild` is a Node.js wrapper script — needs `node` installed. Use `@esbuild/linux-x64` package for standalone binary.
- Counter project at `~/.local/share/celld-test/counter/`, dinit service `celld-dev-counter` on port 8787 (0.0.0.0)
- **Worker syntax gotcha**: Do NOT `extends DurableObject`. Plain class with `constructor(state, env)` + `fetch(request)`. Use `this.state.storage.get()/put()` for persistence.

### Correct Worker Pattern (from official counter example)

```js
export class Counter {
  constructor(state, env) {
    this.state = state;
  }
  async fetch(request) {
    let n = (await this.state.storage.get("n")) ?? 0;
    n++;
    await this.state.storage.put("n", n);
    return Response.json({ n, url: request.url });
  }
}
export default {
  async fetch(request, env) {
    const name = new URL(request.url).searchParams.get("name") ?? "default";
    const id = env.COUNTER.idFromName(name);
    return env.COUNTER.get(id).fetch(request);
  },
};
```

wrangler.jsonc requires `migrations` array with `new_sqlite_classes`.

## Open Questions

- What's the max cell size / write throughput limit?
- How does ownership balancing work with 2 nodes of equal weight?
- Can crabjar talk to celld cells directly (HTTP API) or does it need a worker wrapper?
- What happens to in-flight requests during cell migration?
- Is there a way to query across cells (or is it strictly per-cell isolation)?
