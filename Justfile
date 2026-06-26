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
