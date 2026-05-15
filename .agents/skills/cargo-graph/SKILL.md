---
name: cargo-graph
description: |
  Use this skill whenever you need to reason about the dependency graph of a Rust project — what is declared vs what actually compiled, which transitive crates snuck in, or which declared dependencies are orphaned (optional/inactive). Trigger when the user asks about Cargo.toml or Cargo.lock in any repo, asks "what pulled in X", asks about transitive dependencies, wants to audit a workspace's dependency surface, or asks why a crate is or isn't in the compiled graph. Also trigger when running cargo-declared analysis, checking dependency drift, or preparing a dependency audit for ingestion into mirror-log.
---

# Cargo Graph

Reasons about the declared-vs-compiled dependency gap for any Rust workspace or crate. Built on `cargo-declared` logic — analyze Cargo.toml vs Cargo.lock to identify transitive crates, orphaned dependencies, and the compiled gap.

## Core Concepts

Before acting, internalize these three sets:

| Set | Source | What it means |
|---|---|---|
| **declared** | `Cargo.toml` `[dependencies]` | What the author explicitly asked for |
| **compiled** | `cargo metadata` resolve graph | What Cargo actually resolved and built |
| **delta** | compiled − declared | Transitive crates — implicit contracts nobody signed |
| **orphaned** | declared − compiled | Declared but inactive (optional features, inactive targets) |

**Correctness invariant** — this must always hold:
```
compiled_count == declared_count - orphaned_count + delta_count
```

If it doesn't hold after any analysis, surface the discrepancy before drawing conclusions.

**BFS shortest-predecessor** — `via` attribution traces the shortest path from the root package to a transitive crate. A crate pulled in by multiple paths is attributed to the one closest to root. This means `via` shows the *nearest* declared dependency, not necessarily the only one.

**Composite key** — crates are uniquely identified by `name + version + source`. The same crate name at two different semver versions is two distinct entries. Always include version when discussing a specific crate.

---

## Workflow

### 1. Locate the target

Accept a path from the user. It can be:
- A `Cargo.toml` file directly
- A directory containing a `Cargo.toml`
- A workspace root (analyzes the workspace member at that path, not the whole workspace)

If no path is given, default to the current working directory.

### 2. Read the manifest and lockfile

Read both files before reasoning:

```
Read: <path>/Cargo.toml
Read: <path>/Cargo.lock   (if present)
```

`Cargo.toml` gives the declared set. `Cargo.lock` gives the fully resolved compiled set including all transitive versions. If `Cargo.lock` is absent (library crate), note this — the compiled set is determined by the consumer's resolution, not the library itself.

### 3. Run cargo-declared (if the tool is installed)

If `cargo-declared` is installed in the environment, prefer running it over manual analysis:

```bash
cargo declared --path <path> --json
```

Parse the JSON output — keys are `declared`, `compiled`, `delta`, `orphaned`, `summary`. See `references/output-schema.md` for the full schema.

If `cargo-declared` is not installed, perform manual analysis from the files (see Step 4).

### 4. Manual analysis (fallback)

When `cargo-declared` is not available, derive the sets manually:

**Declared set** — read `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` from `Cargo.toml`. Note each entry's name, version req, and kind.

**Compiled set** — read `Cargo.lock`. The `[[package]]` entries list every resolved crate. Filter out the root package itself. Each entry is a compiled dependency.

**Delta** — compiled entries whose names do not appear as declared keys. For each, trace the `via` path: find which declared dependency has this crate as a transitive dependency. Use the `dependencies` field in `Cargo.lock` `[[package]]` entries to walk the graph.

**Orphaned** — declared entries whose names do not appear in any `[[package]]` entry in `Cargo.lock`. Most commonly optional features that are not activated.

### 5. Answer the user's question

Frame the answer around the three sets. Always include:
- The counts (declared / compiled / delta / orphaned)
- The invariant check result
- Specific crates the user asked about, with `via` attribution if transitive
- Any orphaned entries and why they are likely inactive

### 6. Flag uncertainty

Every answer must include what the analysis might have missed:
- Workspace members not analyzed (if this is a workspace root)
- Platform-specific or feature-gated dependencies that may not appear in the current lock resolution
- Version staleness (if `Cargo.lock` is old or absent)
- Whether the invariant was verified or assumed

---

## Common Questions and How to Answer Them

**"What pulled in X?"**
Find X in the compiled set. Walk the `Cargo.lock` dependency graph backward from X to the first declared dependency. That is the `via` attribution. If multiple paths exist, the shortest one (fewest hops from root) is the canonical answer.

**"Why is X not in the compiled graph?"**
Check if X is declared. If yes, it is likely orphaned — an optional dependency with no active feature enabling it, or a dev-dependency in a context where dev-deps are not resolved. If X is not declared, it was never asked for.

**"Is X a direct or transitive dependency?"**
Direct = appears in `Cargo.toml` declared set. Transitive = appears in compiled set but not declared set (it is in the delta).

**"What is the full transitive closure of X?"**
Walk the `Cargo.lock` `dependencies` field for X's `[[package]]` entry recursively. List all reachable packages. Note that this is the closure of X, not the closure of the whole project.

**"What changed between these two lock files?"**
Compare `[[package]]` entries. New entries = added to compiled set. Removed entries = dropped from compiled set. Version changes = upgrades or downgrades. Correlate changes against `Cargo.toml` diff to distinguish intentional declared changes from transitive drift.

---

---

---

## Constraints

- This skill is **read-only**. It reads manifests and lockfiles. It does not modify them.
- Do not run `cargo build` or `cargo update` — those are action-layer operations.
- Do not infer what *should* be declared. Surface what *is* declared vs what compiled. The gap is information, not an error to fix unless the user asks you to fix it.
- If the invariant fails, report it as a finding, not a bug in the tool. It may indicate a workspace-mode edge case or a feature-gated resolution the analysis did not account for.
