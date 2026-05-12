# Phase 0 — Safe Adoption

Zero-risk items: adopt existing in-tree types/macros. No new code written.

## Items

### CC-8 · Adopt `pretty_fmt!` / `pretty_errorln!` macros

- **Crates:** install
- **Sites:** ~18
- **Effort:** S
- **Unsafe removed:** 0
- **Recipe:** `pretty_fmt!` is already a proc-macro in `src/bun_core_macros/lib.rs`, wrapped at `bun_core/output.rs:1160-1176`. Replace `// TODO(port): Output.prettyFmt` sites in `install/lockfile/printer/tree_printer.rs` etc. with the existing macro. Do NOT write a new `macro_rules!`.
- **Status:** pending

### CC-2 · Adopt `MultiArrayList::split_mut()` for SoA disjoint-column borrows

- **Crates:** install, bundler
- **Sites:** ~42 (install 14; bundler ~28)
- **Effort:** S
- **Unsafe removed:** ~41
- **Recipe:** `multi_array_columns!` already generates `split_mut() -> XxxMut<'_>` and `split_raw()`. Replace remaining `unsafe { from_raw_parts_mut(slice.items_raw::<"field",T>(), len) }` and per-iteration re-borrow chains with the generated accessor.
- **Status:** pending

### CC-1 · Adopt `bun_ptr::BackRef<T>` for all parent/back-pointer raw fields

- **Crates:** runtime, jsc, install, bundler
- **Sites:** ~100 (`*mut Self` C-callback thunks ~45; jsc backrefs 12; install `*mut PackageManager`/`*mut Log`/`NonNull<DotEnv>` 25; bundler `LinkerContext.resolver`/`Worker.ctx` ~18)
- **Effort:** M
- **Unsafe removed:** ~50 deref sites
- **Recipe:** Retype every `*mut Parent` / `Option<NonNull<Parent>>` field encoding "child outlived by pinned singleton parent" as `BackRef<Parent>` (already `#[repr(transparent)] NonNull<T>` + `Deref<Target=T>` + `Copy` + `unsafe get_mut()` at `src/ptr/lib.rs:96`). Callers change `unsafe { &*self.pm }` → `&*self.pm`.
- **Status:** pending

## Anchor Crate: install

I-1 through I-6 cover CC-1, CC-2, CC-8, and I-4 error narrowing (~166 sites total). Good as first crate to prove the patterns.

| ID | Title | Sites | Effort | Unsafe− | Status |
|---|---|---|---|---|---|
| I-1 | Adopt `BackRef<PackageManager>` for install backrefs (see CC-1) | 25 | M | ~25 | pending |
| I-2 | Adopt `split_mut()` for install SoA borrows (see CC-2) | 14 | S | ~14 | pending |
| I-3 | Fold `|d| d.close()` scopeguard sites into install resource Drop | 1 | S | ~1 | pending |
| I-4 | Narrow install error set to `InstallError` enum (see CC-4) | 96 | L | 0 | pending |
| I-5 | install lockfile `Wyhash11` preservation — do NOT swap to `Wyhash` | ~10 | S | 0 | pending |
| I-6 | adopt `pretty_fmt!` for install output (see CC-8) | ~18 | S | 0 | pending |
