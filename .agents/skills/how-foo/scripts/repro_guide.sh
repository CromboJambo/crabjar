#!/usr/bin/env bash
# generate reproduction guide from reference directory
set -euo pipefail

TARGET_PATH="${1:-}"

if [[ -z "$TARGET_PATH" ]]; then
    echo '{"error": "target path required", "usage": "repro_guide.sh <path"}'
    exit 1
fi

if [[ ! -d "$TARGET_PATH" ]]; then
    echo '{"error": "directory not found", "path": "'"$TARGET_PATH"'"}'
    exit 1
fi

if [[ ! -f "$TARGET_PATH/Cargo.toml" ]]; then
    echo '{"error": "Cargo.toml not found — insufficient data", "path": "'"$TARGET_PATH"'"}'
    exit 1
fi

PROJECT_NAME="$(basename "$TARGET_PATH")"
REPRO_FILE="$TARGET_PATH/REPRO.md"

Cargo_toml="$(cat "$TARGET_PATH/Cargo.toml")"

# Extract key deps
deps="$(echo "$Cargo_toml" | grep -A1 '\[dependencies\]' | grep 'version' | sed 's/.*version = "\(.*\)".*/\1/' | head -20 || echo 'none')"

# Extract MSRV
msrv="$(echo "$Cargo_toml" | grep 'msrv' | sed 's/.*msrv = "\(.*\)".*/\1/' || echo 'not declared')"

# Extract release profile
release="$(echo "$Cargo_toml" | grep -A5 'release' || echo 'not declared')"

cat <<EOF > "$REPRO_FILE"
# Reproducing the ${PROJECT_NAME} ${TARGET_PATH} workflow

## Source
The directory ${TARGET_PATH} contains ${PROJECT_NAME}.

## Reproduction Steps
1. Clone/install/build commands
2. Usage commands

## Key Dependencies (Cargo.toml)
| Crate | Version | Purpose |
$(echo "$deps" | while read v; do echo "| crate | $v | TBD |"; done)

## MSRV
${msrv}

## Release Profile
${release}

## Notes
Edge cases, platform restrictions, branch info.
EOF

echo '{"status": "written", "path": "'"$REPRO_FILE"'"}'
