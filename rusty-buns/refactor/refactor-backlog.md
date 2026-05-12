# State Doc: Refactor Backlog

Source: `github.com/oven-sh/bun` branch `claude/phase-a-port` — `docs/PORT_NOTES_PLAN.md`

## Summary — PORT NOTE counts by category, all crates

| Category | runtime | jsc | css | install | sql | bundler | js_parser | **Total** |
|---|---|---|---|---|---|---|---|---|
| explanatory | 1278 | 361 | 176 | 314 | 52 | 319 | 176 | **2676** |
| known_todo | 1059 | 182 | 156 | 296 | 84 | 152 | 146 | **2075** |
| borrowck_workaround | 664 | 53 | 52 | 206 | 5 | 111 | 60 | **1151** |
| missing_idiom | 194 | 5 | 306 | 53 | 0 | 47 | 37 | **642** |
| lifetime_erasure | 213 | 33 | 90 | 30 | 4 | 43 | 30 | **443** |
| bitwise_copy_hazard | 19 | 10 | 5 | 11 | 0 | 11 | 7 | **63** |
| other | 51 | 0 | 0 | 0 | 0 | 19 | 0 | **70** |
| **Total** | **3478** | **644** | **785** | **910** | **145** | **702** | **456** | **7120** |

`explanatory` notes are documentation-only; excluded from backlog.

**Total distinct refactors: 42** (8 cross-cutting + 34 per-crate). Estimated `unsafe` blocks retired if all land: ~520.

## Cross-Cutting Refactors (CC-1 through CC-8)

### CC-1 · Adopt `bun_ptr::BackRef<T>` for all parent/back-pointer raw fields

| Crates | runtime, jsc, install, bundler |
|---|---|
| Sites | ~100 (`*mut Self` C-callback thunks ~45; jsc backrefs 12; install `*mut PackageManager`/`*mut Log`/`NonNull<DotEnv>` 25; bundler `LinkerContext.resolver`/`Worker.ctx` ~18) |
| Effort | M |
| Unsafe removed | ~50 deref sites |
| Recipe | Retype every `*mut Parent` / `Option<NonNull<Parent>>` field encoding "child outlived by pinned singleton parent" as `BackRef<Parent>` (already `#[repr(transparent)] NonNull<T>` + `Deref<Target=T>` + `Copy` + `unsafe get_mut()` at `src/ptr/lib.rs:96`). Callers change `unsafe { &*self.pm }` → `&*self.pm`. |
| Feedback | jsc proposal "introduce `Backref<T>`" was a duplicate — type exists. install proposal "thread `'pm` lifetime" rejected: `&'pm mut` is aliased-mut UB across HTTPThread/thread-pool. bundler proposal split — stored fields use `BackRef`, transient params use `TranspilerCtx<'a>`. runtime `JsCell` proposal split — C-callback user-data thunks go here; JS-re-entrant receivers are R-2. |

### CC-2 · Adopt `MultiArrayList::split_mut()` for SoA disjoint-column borrows

| Crates | install, bundler |
|---|---|
| Sites | ~42 (install 14; bundler ~28) |
| Effort | S |
| Unsafe removed | ~41 |
| Recipe | `multi_array_columns!` already generates `split_mut() -> XxxMut<'_>` and `split_raw()`. Replace remaining `unsafe { from_raw_parts_mut(slice.items_raw::<"field",T>(), len) }` and per-iteration re-borrow chains with the generated accessor. |
| Feedback | Both crates' surveys proposed _building_ this — it exists. Downgraded M→S, scope is caller migration only. |

### CC-3 · Replace `scopeguard` defer-transliterations with typed owning Drop guards

| Crates | runtime, jsc, install, bundler |
|---|---|
| Sites | ~90 (runtime 73; jsc 8; bundler ~8; install 1) |
| Effort | M |
| Unsafe removed | ~20 |
| Recipe | Classify each defer site: (a) cleanup tied to resource → give resource `impl Drop`; (b) cleanup needs `&mut self` that body also borrows → FlushOnDrop pattern (`ConsoleObject.rs`). Do NOT stash `*mut T` alongside live `&mut T` (Stacked Borrows UB). Do NOT blanket-recommend `scopeguard::defer!` — in-tree PORT NOTEs at `ParseTask.rs:840/1132/2060` document why it can't capture there. |
| Feedback | jsc `defer_mut!(*mut T, F)` design rejected as unsound. install count corrected 12→1. bundler count corrected ~20→~8. |

### CC-4 · Narrow `anyhow`/`bun_core::Error` returns to per-module typed enums

| Crates | runtime, jsc, install, sql, bundler, js_parser |
|---|---|
| Sites | ~351 (runtime 132; install 96; sql 52; js_parser 29; bundler 26; jsc 16) |
| Effort | L |
| Unsafe removed | 0 |
| Recipe | One `#[derive(thiserror::Error)]` enum per logical module (`InstallError`, `BundlerError`, `LinkerError`, `ParseError`, `BtjsError`, …) with variants matching Zig error sets. **Exception — sql**: do NOT introduce new enums or `thiserror`; `AnyPostgresError`/`AnyMySQLError` already exist and must round-trip variant names through `IntoStaticStr` for JS `error.code` / snapshot stability. Work is signature-narrowing only: `Result<_, bun_core::Error>` → `Result<_, AnyPostgresError>` + delete lossy `From<bun_core::Error>` round-trip at `AnyPostgresError.rs:66-69`. |
| Feedback | sql recipe corrected per `AnyPostgresError.rs` header comment. |

### CC-5 · Relocate inline `extern "C"` blocks into `*_sys` crates

| Crates | runtime, jsc |
|---|---|
| Sites | ~156 explicit-TODO (runtime 97; jsc 59 — true population closer to 280) |
| Effort | L |
| Unsafe removed | 0 (the `unsafe extern` stays; it just moves) |
| Recipe | Mechanically relocate existing hand-written `extern "C"` blocks verbatim into hand-curated `bun_jsc_sys` / `runtime_sys` / `bun_cares_sys` / `spng_sys` crates, grouped by upstream header. Do NOT use bindgen — JSC/WebCore C++ headers (templates, namespaces, inline methods) are not bindgen-consumable; existing decls are already stable C-ABI shims. First pass: only ~156 with explicit `TODO(port): move to *_sys` markers. |
| Feedback | "bindgen-generated" suggestion dropped. |

### CC-6 · Replace manual `.deref()`/`.deinit()` with `RefPtr<T>` / `CppOwned<T>` / `impl Drop`

| Crates | runtime, jsc |
|---|---|
| Sites | ~97 (runtime 92 "Zig defer x.deref() → Drop"; jsc 5 ref-counted C++ ctors) |
| Effort | M |
| Unsafe removed | ~13 |
| Recipe | Audit each site: where field type already has `Drop`, delete comment and redundant manual call. Where it doesn't, add `impl Drop` (Rust-owned) or wrap in `CppOwned<T>(NonNull<T>)` whose `Drop` calls FFI `deref`/`destroy` (FetchHeaders, RegularExpression, URL, TextCodec, JSCArrayBuffer). Ref-counted types (FetchHeaders/AbortSignal/DOMFormData/URLSearchParams) are WTF::RefCounted, NOT GC cells — belong here, not in J-2 `GcPtr`. |
| Feedback | Ref-counted types moved out of `GcPtr` bucket. |

### CC-7 · Lifetime threading for `'static` / `*const` placeholder fields — gated by heap-ownership classification

| Crates | runtime, css, sql, js_parser (bundler explicitly excluded — see B-1) |
|---|---|
| Sites | ~183 (runtime 81; css 85; js_parser 13; sql 4) |
| Effort | XL |
| Unsafe removed | ~150 |
| Recipe | **Mandatory step 1**: classify every flagged struct as (a) stack-/arena-scoped → add `<'bump>`/`<'i>`/`<'a>` param; or (b) JS-heap-owned (`#[bun_jsc::JsClass]`, stored in GC cell) → must stay `'static`; wrap raw field in `JsBuffer`/`BackRef` newtype with documented invariant. ~25 runtime candidates (FileReader, FormData, ByteStream, blob CopyFile) are JsClass-backed and cannot take a lifetime param. css: thread `Parser`'s existing `'a` into AST nodes as `'i`. bundler boundary keeps `StyleSheet<'static, _>` (bump arena IS source owner). sql: `Data<'a>` with `Temporary(&'a [u8]); redesign self-referential `Data::slice_z()` to return `&[u8]` borrowing `&self`; ~15 packet structs gain `<'a>`. No monomorphization concern — lifetime params erase before codegen. |
| Feedback | runtime JsClass constraint added. css "add `'i` to Parser" corrected (it's already there). bundler `'arena` threading rejected entirely (heterogeneous per-worker bumps). sql `slice_z` self-ref + ~15-struct blast radius noted. |

### CC-8 · Adopt existing `pretty_fmt!` / `pretty_errorln!` macros

| Crates | install |
|---|---|
| Sites | ~18 |
| Effort | S |
| Unsafe removed | 0 |
| Recipe | `pretty_fmt!` is already a proc-macro in `src/bun_core_macros/lib.rs`, wrapped at `bun_core/output.rs:1160-1176`. Replace `// TODO(port): Output.prettyFmt` sites in `install/lockfile/printer/tree_printer.rs` etc. with the existing macro. Do NOT write a new `macro_rules!` — that would be a third implementation. |
| Feedback | Downgraded M→S; reframed as adoption. |

### CC-9 deferred

`impl Drop for bun_string::String` deferred until after port-branch merge.

## Per-Crate Refactors

### runtime (R-1 through R-5)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| R-1 | Migrate `expect/to*.rs` to existing `PostMatchGuard` | 53 | XS | 0 |
| R-2 | JS-re-entrant receivers: `&Self` + per-field `Cell`/`RefCell` (NOT whole-struct `&mut`) | ~45 | XL | ~60 |
| R-3 | Split `DevServer` into disjoint sub-structs (`Bundles`/`Graph`/`Assets`/`Io`) | 45 | L | ~40 |
| R-4 | Fold `expect/to*.rs` host fns into `impl Expect` for `#[host_fn(method)]` | 50 | M | 0 |
| R-5 | `JSValue::to_fmt` formatter sharing — rescope or drop | 39 | S | 0 |

**R-2 status (2026-05-11)**: noalias-hunt enumerated 277 candidates → 73 survived 2-vote triage → 23 ASM-verified PROVEN_CACHED in release x86_64. All 23 have Phase-0 `black_box` launders; 5 types have full Phase-2 `&self`+Cell migration (HTMLRewriter, NodeHTTPResponse, TimerObjectInternals, FileSink, ServerWebSocket). Phase-1 codegen `sharedThis` flag lets remaining 62 types opt in incrementally. 50 NOT_CACHED-SUSPECT sites are still language UB.

### jsc (J-1 through J-5)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| J-1 | Implement `#[bun_jsc::host_fn]` / `#[derive(PojoFields)]` proc-macro family | 17 | L | ~25 |
| J-2 | `GcPtr<T>` for true `JSCell` subclasses only, with `NoGc<'_>` token | ~7 | M | ~10 |
| J-3 | `VirtualMachine` split-borrow: stop materializing `&mut VirtualMachine` | 18 | L | ~40 |
| J-4 | `SavedSourceMap` RAII `SourceMapGuard<'_>` | 4 | S | 2 |
| J-5 | `impl Clone for ConsoleObject::Formatter` | 4 | S | 4 |

### css (C-1 through C-9)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| C-1 | Apply existing `#[derive(DeepClone, CssEql, CssHash)]` to remaining hand-expansions | 106 | M | 0 |
| C-2 | Adopt `bun_collections::EnumMap`/`EnumSet` for CSS property dispatch | ~20 | S | 0 |
| C-3 | `StyleSheet` lifetime threading (see CC-7 — bundler excluded) | 85 | XL | ~150 |
| C-4 | Delete `implement_deep_clone`/`implement_eql` free-fn shims once zero callers remain | ~10 | S | 0 |
| C-5 | CSS parser `<'i>` lifetime into AST nodes (see CC-7) | ~15 | M | ~5 |
| C-6 | CSS derive crate cleanup (see C-1) | ~5 | S | 0 |
| C-7 | CSS selector match vtable → enum dispatch | ~30 | M | ~10 |
| C-8 | CSS media query parser lifetime threading | ~10 | S | 0 |
| C-9 | CSS font-face parsing error set narrowing | ~5 | S | 0 |

### install (I-1 through I-6)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| I-1 | Adopt `BackRef<PackageManager>` for install backrefs (see CC-1) | 25 | M | ~25 |
| I-2 | Adopt `split_mut()` for install SoA borrows (see CC-2) | 14 | S | ~14 |
| I-3 | Fold `|d| d.close()` scopeguard sites into install resource Drop | 1 | S | ~1 |
| I-4 | Narrow install error set to `InstallError` enum (see CC-4) | 96 | L | 0 |
| I-5 | install lockfile `Wyhash11` preservation — do NOT swap to `Wyhash` | ~10 | S | 0 |
| I-6 | adopt `pretty_fmt!` for install output (see CC-8) | ~18 | S | 0 |

### sql (S-1 through S-5)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| S-1 | sql error set narrowing — signature only, no new enums (see CC-4 exception) | 52 | L | 0 |
| S-2 | `Data<'a>` self-referential redesign (`slice_z()` → `&[u8]` borrowing `&self`) | 4 | XL | ~4 |
| S-3 | ~15 packet structs gain `<'a>` lifetime | ~15 | XL | ~15 |
| S-4 | SQL `IntoStaticStr` round-trip preservation for JS `error.code` | ~10 | S | 0 |
| S-5 | delete lossy `From<bun_core::Error>` at `AnyPostgresError.rs:66-69` | 1 | S | 0 |

### bundler (B-1 through B-7)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| B-1 | bundler `'arena` threading rejected entirely (heterogeneous per-worker bumps) | — | — | — |
| B-2 | bundler transient params use `TranspilerCtx<'a>` (see CC-1 split) | ~18 | M | ~18 |
| B-3 | Adopt `split_mut()` for bundler SoA borrows (see CC-2) | ~28 | S | ~28 |
| B-4 | Narrow bundler error set to `BundlerError`/`LinkerError` enums (see CC-4) | 26 | L | 0 |
| B-5 | relocating bundler `extern "C"` into `*_sys` (see CC-5) | ~10 | L | 0 |
| B-6 | bundler worker arena Drop ordering (see CC-6) | ~5 | M | ~5 |
| B-7 | bundler `pretty_fmt!` adoption (see CC-8) | ~5 | S | 0 |

### js_parser (P-1)

| ID | Title | Sites | Effort | Unsafe− |
|---|---|---|---|---|
| P-1 | js_parser error set narrowing + lifetime threading (see CC-4 + CC-7) | 29 + 13 | L + XL | ~13 |

## Recommended Order

12 prioritized steps + background items, ordered by leverage × inverse risk.

1. CC-8 (S, adoption — zero risk)
2. CC-2 (S, adoption — zero risk)
3. CC-1 (M, adoption — low risk)
4. CC-3 (M, Drop guards — medium risk)
5. CC-4 (L, error narrowing — low risk)
6. CC-5 (L, extern relocation — zero risk)
7. CC-6 (M, RAII cleanup — medium risk)
8. CC-7 (XL, lifetime threading — high risk, gated)
9. R-2 Phase-2 remaining (XL, JS re-entrant — high risk)
10. J-1 (L, proc-macro family — medium risk)
11. J-3 (L, VM split-borrow — medium risk)
12. R-3 (L, DevServer split — medium risk)

Background:
- C-1, C-4 (CSS derive cleanup)
- I-5 (Wyhash11 preservation)
- S-4 (SQL `IntoStaticStr` preservation)
- CC-9 deferred (`impl Drop for bun_string::String`)
