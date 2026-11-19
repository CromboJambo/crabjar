# Crabjar Crate Publishing Strategy

Status: Spec — publishing leaf crates to crates.io for external consumption
Scope: All 13 self-contained leaf crates + dependency-ordered publishing of internal crates
Prerequisites: Crates must be self-contained (no path dependencies on unpublished crates)

## §0 Goal

Make crabjar's utility crates available on crates.io so that:

1. External projects can depend on specific capabilities without pulling the full workspace
2. Git vendoring (`branch = "main"`) becomes an option rather than a requirement
3. Version pinning and semantic versioning are enforced for published APIs
4. The ecosystem benefits from crabjar's well-tested components (guard, web-search, etc.)

## §1 Publishing Tiers

### Tier 1: Leaf Crates (publish immediately)

These 13 crates have zero internal dependencies and can be published independently:

| Crate | Description | External Deps Count | Priority |
|-------|-------------|---------------------|----------|
| `crabjar-guard` | Resource enforcement and policy gating | ~8 | **High** (already used by riod, tuirowser) |
| `crabjar-web-search` | Multi-backend web search (SearXNG, DDG, Google, Brave) | ~6 | **High** (already used by tuirowser) |
| `crabjar-web-extract` | Web content extraction + PDF support | ~8 | **High** (already used by tuirowser) |
| `crabjar-telemetry` | Observability and metrics collection | ~5 | Medium |
| `crabjar-terminal` | Terminal session management | ~4 | Medium |
| `crabjar-memory` | Memory/session persistence | ~6 | Medium |
| `crabjar-sandbox` | Process sandboxing | ~3 | Medium |
| `crabjar-podman` | Podman container management + GPU passthrough | ~5 | Medium |
| `crabjar-ssh-bastion` | SSH bastion with russh | ~6 | Medium |
| `crabjar-artifact` | Content-addressed artifact storage | ~4 | Low |
| `crabjar-code-audit` | Code audit and analysis | ~5 | Low |
| `crabjar-lab-topology` | Lab topology discovery and mapping | ~3 | Low |
| `crabjar-slow-friend` | Drift detection for LLM inference | ~4 | Low |
| `crabjar-terrarium` | Isometric habitat visualization | ~7 | Low |
| `crabjar-universal-structure` | Universal structural analysis | ~5 | Low |

Note: `crabjar-web-search` and `crabjar-web-extract` are already in the crates list from the tuirowser integration work. Verify they exist as separate crates or need to be split from a combined crate.

### Tier 2: Internal-Dep Crates (publish after dependencies)

These crates depend on other crabjar crates and must be published in topological order:

| Crate | Depends On | Publish After |
|-------|-----------|---------------|
| `crabjar-benchmarks` | guard | guard published |
| `crabjar-scheduler` | telemetry, guard | both published |
| `crabjar-email-smtp` | host-core | host-core published |
| `crabjar-orchestrator` | 13 internal crates | most of workspace published |

### Tier 3: Meta-Crate (optional)

`crabjar` (the main crate) could be published as a meta-crate that re-exports the public API of all sub-crates. This gives users who want "just crabjar" a single dependency. **Not required** — most users will depend on specific crates.

## §2 Pre-Publish Checklist (per crate)

Before publishing each crate:

1. **No path dependencies**: All `crabjar-*` deps must be replaced with versioned registry deps
2. **README.md**: Present at crate root, describes the crate's purpose and usage
3. **LICENSE**: MIT or Apache-2.0 (check workspace-level license)
4. **Repository field**: Points to GitHub with correct path
5. **Description field**: 1-2 sentence description for crates.io search
6. **Keywords**: 3-5 relevant keywords
7. **Version**: Starts at 0.1.0 (pre-1.0, API may change)
8. **Tests**: `cargo test` passes in isolation (not just in workspace)
9. **Docs**: `cargo doc` builds without errors; public API is documented
10. **MSRV**: Minimum Supported Rust Version declared and tested

### 2.1 Cargo.toml Template for Publishing

```toml
[package]
name = "crabjar-guard"
version = "0.1.0"
edition = "2024"
description = "Resource enforcement and policy gating for agent execution environments"
license = "MIT OR Apache-2.0"
repository = "https://github.com/CromboJambo/crabjar/tree/main/crates/guard"
keywords = ["agents", "security", "policy", "resources", "enforcement"]
categories = ["development-tools", "network-programming"]
readme = "README.md"

[dependencies]
# Only registry dependencies — no path deps
anyhow = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["sync"] }
# ... etc

[dev-dependencies]
criterion = "0.5"  # if benchmarks exist
```

### 2.2 Required Fields Check

For each crate, verify these fields exist in `[package]`:
- `name` ✅ (all have)
- `version` ✅ (all have)
- `edition` ✅ (all have)
- `description` — **add if missing**
- `license` — **add if missing**
- `repository` — **add if missing**
- `keywords` — **add if missing**
- `categories` — **add if missing**

## §3 Publishing Workflow

### 3.1 Per-Crate Publish Steps

```bash
# 1. Verify the crate builds and tests pass in isolation
cd crates/<crate-name>
cargo build --release
cargo test
cargo doc --no-deps

# 2. Check for path dependencies (should be none for leaf crates)
grep -r "path =" Cargo.toml || echo "No path deps"

# 3. Dry-run publish (checks without uploading)
cargo publish --dry-run

# 4. Publish to crates.io
cargo publish

# 5. Verify on crates.io
curl -s https://crates.io/api/v1/crates/<crate-name> | jq .num_downloads
```

### 3.2 Batch Publishing Order

Publish in this order (dependencies first):

**Round 1 (all independent):**
1. crabjar-guard
2. crabjar-web-search
3. crabjar-web-extract
4. crabjar-telemetry
5. crabjar-terminal
6. crabjar-memory
7. crabjar-sandbox
8. crabjar-podman
9. crabjar-ssh-bastion
10. crabjar-artifact
11. crabjar-code-audit
12. crabjar-lab-topology
13. crabjar-slow-friend
14. crabjar-terrarium
15. crabjar-universal-structure

**Round 2 (after Round 1):**
16. crabjar-benchmarks (needs guard)
17. crabjar-scheduler (needs telemetry, guard)

**Round 3 (later, when ready):**
18. crabjar-email-smtp (needs host-core)
19. crabjar-orchestrator (needs most of workspace)

### 3.3 Versioning Strategy

- **0.x.y**: Pre-stable. Breaking changes allowed in minor versions.
- **1.0.0**: API stability commitment. Only breaking changes in major versions.
- **Publish cadence**: Batch publish after significant feature work, not on every commit.
- **Changelog**: Each crate should have a `CHANGELOG.md` (or use the workspace-level one with per-crate sections).

## §4 Migration from Git Dependencies

Projects currently using git dependencies (`branch = "main"`) can migrate to registry deps once crates are published:

**Before:**
```toml
crabjar-guard = { git = "https://github.com/CromboJambo/crabjar", branch = "main" }
```

**After:**
```toml
crabjar-guard = "0.1"
```

**Migration plan for known consumers:**
- tuirowser: 3 crates (web-search, web-extract, guard) — migrate after Round 1
- riod: 1 crate (guard) — migrate after Round 1
- Others: TBD

**Hybrid approach during transition:**
```toml
# Use registry version by default, git for bleeding-edge features
crabjar-guard = "0.1"
# crabjar-guard = { git = "https://github.com/CromboJambo/crabjar", branch = "main" }  # for dev
```

## §5 Automation

### 5.1 Publish Script

Create `tools/publish.sh` in the crabjar repo:

```bash
#!/usr/bin/env bash
# Publish a crate to crates.io
# Usage: ./tools/publish.sh <crate-name> [--dry-run]

set -euo pipefail

CRATE="${1:?Usage: publish.sh <crate-name> [--dry-run]}"
DRY_RUN=""
[[ "${2:-}" == "--dry-run" ]] && DRY_RUN="--dry-run"

cd "crates/$CRATE"

echo "=== Publishing $CRATE ==="
cargo build --release
cargo test
cargo doc --no-deps 2>&1 | grep -v "^$" || true

if [[ -z "$DRY_RUN" ]]; then
    cargo publish
    echo "Published $CRATE $(cargo pkgid | sed 's/.*@//')"
else
    cargo publish --dry-run
fi
```

### 5.2 CI Integration (optional)

Add a GitHub Actions workflow that:
1. Runs on tags matching `crates/<name>/v*`
2. Publishes the specified crate
3. Requires `CARGO_REGISTRY_TOKEN` secret

For now, manual publishing is sufficient for home-lab scale.

## §6 Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Publishing a crate with broken API | 0.x.y versions allow breaking changes; test thoroughly before publish |
| Forgetting to update version after changes | `cargo publish` fails if version already exists on crates.io |
| Path dependencies slip through | `cargo publish --dry-run` catches these |
| License inconsistency | Standardize on MIT OR Apache-2.0 across all crates |
| Crates.io rate limiting | Batch publishes with delays between crates (crates.io allows 10/min) |

## §7 Success Criteria

- [ ] All 15 Tier 1 leaf crates published to crates.io
- [ ] Each crate has README, description, keywords, license, repository fields
- [ ] `cargo search crabjar` shows all published crates
- [ ] tuirowser and riod migrated from git deps to registry deps
- [ ] Publish script works for repeat publishing
- [ ] Documentation updated: AGENTS.md mentions publishing workflow

## Appendix A: Crate Dependency Graph (for publish ordering)

```
Tier 1 (no internal deps):
  guard ← benchmarks, scheduler
  telemetry ← scheduler
  web-search ← orchestrator
  web-extract ← orchestrator
  terminal ← orchestrator
  memory ← orchestrator
  sandbox ← orchestrator
  podman ← (standalone)
  ssh-bastion ← (standalone)
  artifact ← (standalone)
  code-audit ← (standalone)
  lab-topology ← (standalone)
  slow-friend ← (standalone)
  terrarium ← (standalone)
  universal-structure ← (standalone)

Tier 2:
  benchmarks → guard
  scheduler → telemetry, guard
  email-smtp → host-core (not yet leaf)

Tier 3:
  orchestrator → 13 internal crates
```

## Appendix B: Verification Commands

```bash
# Check all leaf crates for path dependencies
for d in crates/*/; do
  name=$(basename "$d")
  if grep -q "path =" "$d/Cargo.toml" 2>/dev/null; then
    echo "$name: HAS PATH DEPS"
    grep "path =" "$d/Cargo.toml"
  fi
done

# Dry-run publish for all leaf crates
for crate in guard web-search web-extract telemetry terminal memory sandbox podman ssh-bastion artifact code-audit lab-topology slow-friend terrarium universal-structure; do
  echo "=== $crate ==="
  (cd "crates/$crate" && cargo publish --dry-run 2>&1 | tail -5)
done

# Verify published crates
for crate in guard web-search web-extract; do
  echo "$crate: $(curl -s https://crates.io/api/v1/crates/crabjar-$crate | jq -r '.crate.max_version // "NOT PUBLISHED"')"
done
```
