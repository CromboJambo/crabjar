#!/usr/bin/env bash
# hermes-struct-tok-pipeline: watch for Rust source files, tokenize them
set -euo pipefail

WATCH_DIR="${1:-/var/lib/mirror-lab/capture/rust-src}"
STAGING_DIR="/var/lib/mirror-lab/staging/struct-tokens"
mkdir -p "$WATCH_DIR" "$STAGING_DIR"

process_file() {
    local file="$1"
    local basename=$(basename "$file")
    local timestamp=$(date +%s)
    
    echo "[struct-tok-pipeline] Processing: $basename" >&2
    
    # Tokenize the Rust source
    local tokens
    if ! tokens=$(struct-tok encode < "$file" 2>&1); then
        echo "[struct-tok-pipeline] Error tokenizing $basename: $tokens" >&2
        return 1
    fi
    
    # Store as structured event (JSON lines format)
    local event_file="$STAGING_DIR/${timestamp}_${basename}.jsonl"
    echo "{\"ts\":${timestamp},\"type\":\"rust_source\",\"file\":\"${basename}\",\"tokens\":\"${tokens}\"}" > "$event_file"
    
    echo "[struct-tok-pipeline] Stored: ${basename} -> ${timestamp}_${basename}.jsonl" >&2
    return 0
}

# Process any existing files in watch dir
for file in "$WATCH_DIR"/*.rs; do
    if [ -f "$file" ]; then
        process_file "$file" || true
    fi
done

echo "[struct-tok-pipeline] Watcher started, monitoring $WATCH_DIR" >&2
