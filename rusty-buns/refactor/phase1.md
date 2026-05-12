# Phase 1 — New Type / Guard Introduction

Medium-to-high effort: introduce new types, guards, and error sets. Risk: medium.

## Items

### CC-3 · Replace `scopeguard` defer-transliterations with typed owning Drop guards

- **Crates:** runtime, jsc, install, bundler
- **Sites:** ~90 (runtime 73; jsc 8; bundler ~8; install 1)
- **Effort:** M
- **Unsafe removed:** ~20
- **Recipe:** Classify each defer site: (a) cleanup tied to resource → give resource `impl Drop`; (b) cleanup needs `&mut self` that body also borrows → FlushOnDrop pattern (`ConsoleObject.rs`). Do NOT stash `*mut T` alongside live `&mut T` (Stacked Borrows UB). Do NOT blanket-recommend `scopeguard::defer!` — in-tree PORT NOTEs at `ParseTask.rs:840/1132/2060` document why it can't capture there.
- **Status:** pending

### CC-4 · Narrow `anyhow`/`bun_core::Error` returns to per-module typed enums

- **Crates:** runtime, jsc, install, sql, bundler, js_parser
- **Sites:** ~351 (runtime 132; install 96; sql 52; js_parser 29; bundler 26; jsc 16)
- **Effort:** L
- **Unsafe removed:** 0
- **Recipe:** One `#[derive(thiserror::Error)]` enum per logical module (`InstallError`, `BundlerError`, `LinkerError`, `ParseError`, `BtjsError`, …) with variants matching Zig error sets. **Exception — sql**: do NOT introduce new enums or `thiserror`; `AnyPostgresError`/`AnyMySQLError` already exist and must round-trip variant names through `IntoStaticStr` for JS `error.code` / snapshot stability. Work is signature-narrowing only: `Result<_, bun_core::Error>` → `Result<_, AnyPostgresError>` + delete lossy `From<bun_core::Error>` round-trip at `AnyPostgresError.rs:66-69`.
- **Status:** pending

### CC-5 · Relocate inline `extern "C"` blocks into `*_sys` crates

- **Crates:** runtime, jsc
- **Sites:** ~156 explicit-TODO (runtime 97; jsc 59 — true population closer to 280)
- **Effort:** L
- **Unsafe removed:** 0 (the `unsafe extern` stays; it just moves)
- **Recipe:** Mechanically relocate existing hand-written `extern "C"` blocks verbatim into hand-curated `bun_jsc_sys` / `runtime_sys` / `bun_cares_sys` / `spng_sys` crates, grouped by upstream header. Do NOT use bindgen — JSC/WebCore C++ headers (templates, namespaces, inline methods) are not bindgen-consumable; existing decls are already stable C-ABI shims. First pass: only ~156 with explicit `TODO(port): move to *_sys` markers.
- **Status:** pending

### CC-6 · Replace manual `.deref()`/`.deinit()` with `RefPtr<T>` / `CppOwned<T>` / `impl Drop`

- **Crates:** runtime, jsc
- **Sites:** ~97 (runtime 92 "Zig defer x.deref() → Drop"; jsc 5 ref-counted C++ ctors)
- **Effort:** M
- **Unsafe removed:** ~13
- **Recipe:** Audit each site: where field type already has `Drop`, delete comment and redundant manual call. Where it doesn't, add `impl Drop` (Rust-owned) or wrap in `CppOwned<T>(NonNull<T>)` whose `Drop` calls FFI `deref`/`destroy` (FetchHeaders, RegularExpression, URL, TextCodec, JSCArrayBuffer). Ref-counted types (FetchHeaders/AbortSignal/DOMFormData/URLSearchParams) are WTF::RefCounted, NOT GC cells — belong here, not in J-2 `GcPtr`.
- **Status:** pending

## Per-Crate Breakdown

### runtime

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| R-1 | Migrate `expect/to*.rs` to existing `PostMatchGuard` | 53 | XS | 0 | pending |
| R-2 | JS-re-entrant receivers: `&Self` + per-field `Cell`/`RefCell` (NOT whole-struct `&mut`) | ~45 | XL | ~60 | pending |
| R-3 | Split `DevServer` into disjoint sub-structs (`Bundles`/`Graph`/`Assets`/`Io`) | 45 | L | ~40 | pending |
| R-4 | Fold `expect/to*.rs` host fns into `impl Expect` for `#[host_fn(method)]` | 50 | M | 0 | pending |
| R-5 | `JSValue::to_fmt` formatter sharing — rescope or drop | 39 | S | 0 | pending |

### jsc

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| J-1 | Implement `#[bun_jsc::host_fn]` / `#[derive(PojoFields)]` proc-macro family | 17 | L | ~25 | pending |
| J-2 | `GcPtr<T>` for true `JSCell` subclasses only, with `NoGc<'_>` token | ~7 | M | ~10 | pending |
| J-3 | `VirtualMachine` split-borrow: stop materializing `&mut VirtualMachine` | 18 | L | ~40 | pending |
| J-4 | `SavedSourceMap` RAII `SourceMapGuard<'_>` | 4 | S | 2 | pending |
| J-5 | `impl Clone for ConsoleObject::Formatter` | 4 | S | 4 | pending |

### css

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| C-1 | Apply existing `#[derive(DeepClone, CssEql, CssHash)]` to remaining hand-expansions | 106 | M | 0 | pending |
| C-2 | Adopt `bun_collections::EnumMap`/`EnumSet` for CSS property dispatch | ~20 | S | 0 | pending |
| C-3 | `StyleSheet` lifetime threading (see CC-7 — bundler excluded) | 85 | XL | ~150 | pending |
| C-4 | Delete `implement_deep_clone`/`implement_eql` free-fn shims once zero callers remain | ~10 | S | 0 | pending |
| C-5 | CSS parser `<'i>` lifetime into AST nodes (see CC-7) | ~15 | M | ~5 | pending |
| C-6 | CSS derive crate cleanup (see C-1) | ~5 | S | 0 | pending |
| C-7 | CSS selector match vtable → enum dispatch | ~30 | M | ~10 | pending |
| C-8 | CSS media query parser lifetime threading | ~10 | S | 0 | pending |
| C-9 | CSS font-face parsing error set narrowing | ~5 | S | 0 | pending |

### bundler

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| B-2 | bundler transient params use `TranspilerCtx<'a>` (see CC-1 split) | ~18 | M | ~18 | pending |
| B-3 | Adopt `split_mut()` for bundler SoA borrows (see CC-2) | ~28 | S | ~28 | pending |
| B-4 | Narrow bundler error set to `BundlerError`/`LinkerError` enums (see CC-4) | 26 | L | 0 | pending |
| B-5 | relocating bundler `extern "C"` into `*_sys` (see CC-5) | ~10 | L | 0 | pending |
| B-6 | bundler worker arena Drop ordering (see CC-6) | ~5 | M | ~5 | pending |
| B-7 | bundler `pretty_fmt!` adoption (see CC-8) | ~5 | S | 0 | pending |

### sql

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| S-1 | sql error set narrowing — signature only, no new enums (see CC-4 exception) | 52 | L | 0 | pending |
| S-2 | `Data<'a>` self-referential redesign (`slice_z()` → `&[u8]` borrowing `&self`) | 4 | XL | ~4 | pending |
| S-3 | ~15 packet structs gain `<'a>` lifetime | ~15 | XL | ~15 | pending |
| S-4 | SQL `IntoStaticStr` round-trip preservation for JS `error.code` | ~10 | S | 0 | pending |
| S-5 | delete lossy `From<bun_core::Error>` at `AnyPostgresError.rs:66-69` | 1 | S | 0 | pending |

### js_parser

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| P-1 | js_parser error set narrowing + lifetime threading (see CC-4 + CC-7) | 29 + 13 | L + XL | ~13 | pending |
