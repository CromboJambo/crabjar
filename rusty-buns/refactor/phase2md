# Phase 2 — Lifetime Threading (Gated, High Risk)

Highest effort and risk: lifetime threading for `'static`/`*const` placeholder fields. Must be gated by heap-ownership classification.

## Items

### CC-7 · Lifetime threading for `'static` / `*const` placeholder fields — gated by heap-ownership classification

- **Crates:** runtime, css, sql, js_parser (bundler explicitly excluded — see B-1)
- **Sites:** ~183 (runtime 81; css 85; js_parser 13; sql 4)
- **Effort:** XL
- **Unsafe removed:** ~150
- **Recipe:** **Mandatory step 1**: classify every flagged struct as (a) stack-/arena-scoped → add `<'bump>`/`<'i>`/`<'a>` param; or (b) JS-heap-owned (`#[bun_jsc::JsClass]`, stored in GC cell) → must stay `'static`; wrap raw field in `JsBuffer`/`BackRef` newtype with documented invariant. ~25 runtime candidates (FileReader, FormData, ByteStream, blob CopyFile) are JsClass-backed and cannot take a lifetime param. css: thread `Parser`'s existing `'a` into AST nodes as `'i`. bundler boundary keeps `StyleSheet<'static, _>` (bump arena IS source owner). sql: `Data<'a>` with `Temporary(&'a [u8]); redesign self-referential `Data::slice_z()` to return `&[u8]` borrowing `&self`; ~15 packet structs gain `<'a>`. No monomorphization concern — lifetime params erase before codegen.
- **Status:** pending

### R-2 Phase-2 remaining · JS re-entrant `&self` + Cell migration

- **Crates:** runtime
- **Sites:** ~62 (23 PROVEN_CACHED already have Phase-0 `black_box` launders; 5 types have full Phase-2 migration; remaining 62 opt in via Phase-1 `sharedThis` flag)
- **Effort:** XL
- **Unsafe removed:** ~60
- **Recipe:** 50 NOT_CACHED-SUSPECT sites are still language UB. Phase-1 codegen `sharedThis` flag lets remaining 62 types opt in incrementally.
- **Status:** pending

## Background Items

- C-1, C-4 (CSS derive cleanup)
- I-5 (Wyhash11 preservation)
- S-4 (SQL `IntoStaticStr` preservation)
- CC-9 deferred (`impl Drop for bun_string::String`)
