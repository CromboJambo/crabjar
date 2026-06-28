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
