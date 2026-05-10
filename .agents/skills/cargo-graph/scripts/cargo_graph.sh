#!/usr/bin/env bash
# cargo-declared dependency graph analysis
set -euo pipefail

TARGET_PATH="${1:-$PWD}"

if [[ ! -f "$TARGET_PATH/Cargo.toml" ]]; then
    echo '{"error": "Cargo.toml not found", "path": "'"$TARGET_PATH"'"}'
    exit 1
fi

if command -v cargo-declared &>/dev/null; then
    cargo declared --path "$TARGET_PATH" --json
else
    echo '{"error": "cargo-declared not installed", "fallback": "manual analysis from Cargo.toml and Cargo.lock"}'
    exit 1
fi
