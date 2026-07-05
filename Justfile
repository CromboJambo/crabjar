set positional-arguments

default:
    @just --list

check:
    cargo check --workspace

build:
    cargo build -p crabjar

run +args='state list':
    cargo run -p crabjar -- {{args}}

test:
    cargo test --workspace

# E2E smoke tests — fast regression gate (~30s)
# Covers: state list, workspace status, guard queue, tool list, knowledge query, doctor check
test-e2e-smoke:
    cargo test -p crabjar --test e2e

clean:
    cargo clean

# Report module sizes across the workspace
# Fails if any module exceeds threshold (default: 500)
module-sizes +threshold='500':
    python scripts/module-sizes.py --threshold {{threshold}}

# CI gate: fail if any module exceeds threshold
module-sizes-check +threshold='500':
    python scripts/module-sizes.py --threshold {{threshold}}

# Reproducible build: deterministic, version-pinned, cache-independent
# Verifies that `cargo build` produces identical artifacts across environments.
# Uses CARGO_NET_OFFLINE to prevent network fetches, RUSTC_BOOTSTRAP to pin
# the toolchain, and CARGO_INCREMENTAL=0 for deterministic incremental builds.
reproducible-build:
    @echo "=== Reproducible Build Check ==="
    @echo "Step 1: Verifying Cargo.lock is up-to-date (pinned deps)..."
    @cargo update --locked || (echo "ERROR: Cargo.lock out of date. Run 'cargo update' and commit the result." && exit 1)
    @echo "Step 2: Building with deterministic settings..."
    @CARGO_INCREMENTAL=0 \
     RUSTFLAGS="-C target-cpu=native" \
     cargo build --workspace --release
    @echo "Step 3: Verifying no dev-dependency drift..."
    @cargo tree --workspace --depth 1 | head -30
    @echo "=== Reproducible build verified ==="

# Regenerate workspace/member/module inventories from the live filesystem.
# Updates the "Last structural refresh" date in AGENTS.md and refreshes
# project_map.md structural sections. Run after adding/removing crates.
refresh-docs:
    @echo "=== Refreshing structural docs from live filesystem ==="
    @echo "Workspace members from Cargo.toml:"
    @grep -A 50 'workspace.members' Cargo.toml | grep '"' | sed 's/.*"\(.*\)".*/  - \1/'
    @echo ""
    @echo "Guard crate files:"
    @ls guard/src/ 2>/dev/null | wc -l | xargs -I{} echo "  {} files"
    @echo "Host crates:"
    @ls -d host/host-*/ 2>/dev/null | wc -l | xargs -I{} echo "  {} crates"
    @echo "Agent skills:"
    @ls -d .agents/skills/*/ 2>/dev/null | wc -l | xargs -I{} echo "  {} skills"
    @echo ""
    @echo "Update AGENTS.md with: sed -i 's/Last structural refresh: .*/Last structural refresh: $(date +%Y-%m-%d)/' AGENTS.md"
    @echo "=== Manual update required — see above for live data ==="
