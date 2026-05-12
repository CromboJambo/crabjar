# State Doc: Deferred Crates

Source: `github.com/oven-sh/bun` branch `claude/phase-a-port`

## Deferred Areas

These areas are explicitly deferred from the Phase A port. They remain in Zig or C/C++ and are not part of the immediate Rust migration spine.

### shell (22K LOC, 180 comptimes)

- `bun_shell` crate — arena + NodeId architecture
- 180 `comptime` instances that Rust cannot replicate directly
- Deferred until post-merge; requires new Rust comptime-equivalent design
- **What it does**: shell command execution, argument parsing, spawn logic

### bake (12K LOC, moving target)

- `bun_bake` crate — dev server + bundler integration
- Moving target: code evolves during port; stabilization needed before porting
- **What it does**: `bun bake` dev server, HMR, bundle_v2 bake integration, `DevServer` struct
- **Known crash fixes**: 15+ bake/dev-server issues likely fixed by the rewrite already (see crash-leak audit)
- **R-3 refactor**: split `DevServer` into disjoint sub-structs (`Bundles`/`Graph`/`Assets`/`Io`) — ~72 `dev_ptr`-related lines do aliased `&mut` UB; sub-struct split lets borrowck prove non-overlap

### hand-written DOMJIT

- `JSBuffer.cpp`, `FFIObject` — hand-written DOMJIT code
- Lives in C++ layer; no Zig source to translate
- Deferred until DOMJIT architecture is documented and stabilized
- **What it does**: DOM JIT compilation for web DOM operations

## Migration Spine Status

Layer 0 → Layer 1 → Layer 2 → Layer 3 flow:

| Layer | Name | Rule | Status |
|---|---|---|---|
| 0 | primitives (no deps) | P0 critical path · ~15K · hours 0-24 | all todo |
| 1 | native infrastructure | ⛔ MUST NOT depend on bun_jsc · cargo-tree CI gate | all todo |
| 2 | JSC boundary | P0 critical path · everything in §5 lives here | all todo |
| 3 | runtime (.classes.ts impls) | flip per-class: impl "zig" | "rust" | all todo |

### Layer 0 — primitives

| Crate | LOC | What |
|---|---|---|
| `bun_sys` | 4.6K | raw libc/windows-sys, NOT std::fs |
| `bun_str` | ~3K | WTF/Bun/ZigString, repr(C) mirrors, simd FFI |
| `bun_alloc` | ~300 | mimalloc, global_allocator + mi_heap Arena |
| `bun_panic` | ~100 | hook → crash_handler |
| `bun_core` | ~500 | enums, url, semver |

### Layer 1 — native infrastructure

| Crate | LOC | What |
|---|---|---|
| `bun_threadpool` | ~1K | kprotty/zap port, verbatim, no rayon |
| `bun_async` | ~3K | uSockets loop FFI, Task(u64), KeepAlive, Timer, no Future |
| `bun_http` | 18K | h1/h2/h3 client, HTTPThread, pico/lshpack/lsquic FFI |
| `bun_ast` | 77K | lexer/parser/printer, BlockStore, StoreRef, Expr/Stmt |
| `bun_install_core` | 45K | pkg manager, lockfile, NetworkTask, extract |
| `bun_bundler_core` | 24K | BundleV2, Graph, Linker, per-worker mi_heap |
| `bun_css` | 73K → ~15K | re-vendor lightningcss + diffs |
| toml/yaml/jsonc/md | ~16K | leaf parsers, first fleet target |

### Layer 2 — JSC boundary

| Crate | LOC | What |
|---|---|---|
| `bun_jsc` | ~4K | JSValue, JSRef, Strong, host_call, jsc_conv!, CallFrame inline |
| `bun_jsc::sys` | auto-gen | extern decls from cppbind.ts [[ZIG_EXPORT]] |
| codegen emitter | ~250 LOC | P1 — emits ZigGeneratedClasses.rs |

### Layer 3 — runtime

Wave-A · pure compute · fleet-first:
- Glob (200), Hash/MD4/5 (~400), Semver (~300), TOML/YAML/Cron (~2K), Stat/StatFS (~500, + ~20 more data carriers)

Glue (*_jsc — bridges L1 ↔ bun_jsc):
- `bun_http_jsc` (fetch, Method.toJS), `bun_install_jsc` (jsParseLockfile, scanner), `bun_bundler_jsc` (JSBundleCompletionTask, plugins)

Rehearsal ladder · sequential, each proves one primitive:
1. Cron — JSRef lifecycle
2. UDPSocket — KeepAlive, uSockets FFI, MarkedArgs
3. Socket<SSL> — const-generic, Handlers, Rc::into_raw
4. MySQL — EventLoopTimer, AutoFlusher, protocol/binding split
5. Postgres — hasPendingActivity (after Zig refactor)
6. Request/Response/Body — Body.Value, RefPtr<C++>
7. ServerWebSocket — DOMJIT path
8. NodeHTTPResponse — Strong promise, socket-ext lookup
9. RequestContext + Server — <SSL,DEBUG,H3>, slab, last

Long tail (parallel after step 3):
- node/* bindings (~40K · ~34 files), webcore/* (Blob, streams, fetch), sql, valkey, s3 (~20K), Subprocess, FFI, napi (~15K)

### Stays C/C++ (FFI)

- JSC + WebKit (vendored)
- ZigGeneratedClasses.cpp (JSCell wrappers, WriteBarrier, visitChildren)
- uSockets / libuv (event loop)
- pico/lshpack/lsquic/boringssl/libarchive/mimalloc/highway (vendored C)
