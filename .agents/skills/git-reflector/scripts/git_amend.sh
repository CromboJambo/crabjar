#!/usr/bin/env bash
# git commit amend with DecisionBlob
set -euo pipefail

DECISION_PATH="${1:-}"
MIRROR_DECISIONS_DIR="${MIRROR_DECISIONS_DIR:-$PWD/mirror_decisions}"

if [[ -z "$DECISION_PATH" ]]; then
    echo '{"error": "DecisionBlob path required", "usage": "git_amend.sh <path"}'
    exit 1
fi

if [[ ! -f "$DECISION_PATH" ]]; then
    echo '{"error": "file not found", "path": "'"$DECISION_PATH"'"}'
    exit 1
fi

mkdir -p "$MIRROR_DECISIONS_DIR"
DEST="$MIRROR_DECISIONS_DIR/$(basename "$DECISION_PATH")"

cp "$DECISION_PATH" "$DEST"
git add "$DEST"
git commit --amend -m "Add decision from $DEST"

HEAD_HASH="$(git rev-parse HEAD)"
echo '{"status": "committed", "head_hash": "'"$HEAD_HASH"'"}'
