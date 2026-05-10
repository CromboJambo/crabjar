# cargo-declared JSON Output Schema

> Reference for `cargo declared --json` output. Read this when parsing or validating the JSON output from the tool.

## Top-level object

```json
{
  "declared": [ DependencyInfo ],
  "compiled": [ DependencyInfo ],
  "delta":    [ DeltaEntry ],
  "orphaned": [ DependencyInfo ],
  "summary":  Summary
}
```

---

## DependencyInfo

Appears in `declared`, `compiled`, and `orphaned` arrays.

| Field | Type | Notes |
|---|---|---|
| `name` | string | Crate name. For renamed deps (`package = "..."`), this is the **alias** used in `Cargo.toml`, not the underlying package name. |
| `version` | string \| null | For `declared`: the version requirement string (e.g. `"^4"`, `"1"`). For `compiled`: the resolved semver string (e.g. `"4.6.0"`). |
| `source` | string \| null | Registry URL or `"path+/abs/path"` for path deps. `null` for the root package. |
| `kind` | `"normal"` \| `"development"` \| `"build"` | Dependency kind. Kind is propagated through the BFS walk — a normal dep's transitive deps inherit `normal`. |

### Example (declared)
```json
{
  "name": "clap",
  "version": "^4",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "kind": "normal"
}
```

### Example (compiled)
```json
{
  "name": "clap_builder",
  "version": "4.6.0",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "kind": "normal"
}
```

---

## DeltaEntry

Appears in the `delta` array. Represents a crate that compiled but was not explicitly declared.

| Field | Type | Notes |
|---|---|---|
| `name` | string | Crate name |
| `version` | string \| null | Resolved semver version |
| `source` | string \| null | Registry URL or path |
| `via` | string | Name of the **nearest declared dependency** that transitively pulled this crate in (BFS shortest-predecessor). Value is `"unknown"` if the path cannot be traced. |

### Example
```json
{
  "name": "clap_builder",
  "version": "4.6.0",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "via": "clap"
}
```

---

## Summary

| Field | Type | Notes |
|---|---|---|
| `declared_count` | integer | Length of the `declared` array |
| `compiled_count` | integer | Length of the `compiled` array |
| `delta_count` | integer | Length of the `delta` array |
| `orphaned_count` | integer | Length of the `orphaned` array |

### Correctness invariant
```
compiled_count == declared_count - orphaned_count + delta_count
```

This invariant is enforced by `output::validate_invariant` in the source. If the invariant fails on real output, it indicates a workspace-mode edge case or feature-gated resolution not covered by the analysis.

---

## Multi-version packages

When the same crate name appears at two different versions (e.g. `shared 0.1.0` and `shared 0.2.0`), both appear as separate entries in `compiled` and `delta`. They are disambiguated by the composite key `name + version + source`. Do not deduplicate by name alone.

## Renamed dependencies

If a dependency uses `package = "underlying-name"` with a local alias, the `name` field in `declared` and `orphaned` is the **alias** (e.g. `serde_alias`), not the underlying package name (`dep-pkg`). In `compiled`, the name is the underlying package name as it appears in the registry.
