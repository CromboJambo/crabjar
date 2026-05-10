#!/usr/bin/env bash
# create DecisionBlob JSON from staged event
set -euo pipefail

STAGED_PATH="${1:-}"
REASON="${2:-}"

if [[ -z "$STAGED_PATH" ]]; then
    echo '{"error": "staged event path required", "usage": "decision_blob.sh <path> [reason"}'
    exit 1
fi

if [[ ! -f "$STAGED_PATH" ]]; then
    echo '{"error": "file not found", "path": "'"$STAGED_PATH"'"}'
    exit 1
fi

CONTENT="$(cat "$STAGED_PATH")"
UUID="$(uuidgen)"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat <<EOF
{
  "uuid": "$UUID",
  "set_at": "$DATE",
  "selected_reflection": "$CONTENT",
  "context_tags": [],
  "kernel_name": "unknown",
  "reason": "$REASON",
  "source": "$STAGED_PATH",
  "provenance": {
    "created_at": "$DATE",
    "immutable": true
  }
}
EOF
