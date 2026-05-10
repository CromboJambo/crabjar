#!/usr/bin/env bash
# generate state doc from directory path
set -euo pipefail

TARGET_PATH="${1:-$PWD}"
OUTPUT_DIR="${OUTPUT_DIR:-$HOME/.mirror-lab/crabjar/state-docs}"
mkdir -p "$OUTPUT_DIR"

if [[ ! -d "$TARGET_PATH" ]]; then
    echo '{"error": "directory not found", "path": "'"$TARGET_PATH"'"}'
    exit 1
fi

PROJECT_NAME="$(basename "$TARGET_PATH")"
STATE_DOC="$OUTPUT_DIR/${PROJECT_NAME}-state.md"

# Gather source data
tree_output="$(find "$TARGET_PATH" -maxdepth 3 -type f | sort)"
cargo_toml="$(cat "$TARGET_PATH/Cargo.toml" 2>/dev/null || echo 'not found')"
readme="$(cat "$TARGET_PATH/README.md" 2>/dev/null || echo 'not found')"
agents_md="$(cat "$TARGET_PATH/AGENTS.md" 2>/dev/null || echo 'not found')"

# Write state doc
cat <<EOF > "$STATE_DOC"
# ${PROJECT_NAME}-state.md

> Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
> Source: $TARGET_PATH
> Purpose: Human-level review for stateful memory approximation → SQLite indexing

---

## 1. Overview

Brief description of what the project is, its core value proposition, version/current state.

---

## 2. Architecture

### 2.1 Workspace Layout
$(echo "$tree_output" | head -50)

### 2.2 Core Components
Table of key components with role and status.

---

## 3. Build & Test

Commands for build, test, lint, benchmarks.

---

## 4. Code Quality & Style

Rules, guidelines, style patterns if applicable.

---

## 5. Crabjar Context

### 5.1 Architecture Alignment
Table mapping components to Crabjar's role (Pure observer, append-only, gated, etc.).

### 5.2 Integration Points
Patterns from this project that crabjar could adopt.

---

## 6. Confidence Assessment

### 6.1 What This Review Captures
List what the review captures from the source data.

### 6.2 What This Review Might Have Missed
List what might have been missed (unaccessible data, assumptions gaps).

### 6.3 Stale After
List conditions that would make this review stale.

---

*End of review.*
EOF

echo '{"status": "written", "path": "'"$STATE_DOC"'"}'
