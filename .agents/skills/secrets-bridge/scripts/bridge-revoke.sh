#!/usr/bin/env bash
# bridge-revoke.sh — Revoke a session's access
# Usage: bridge-revoke.sh [session_id]
# If no session_id given, revokes active session

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
KEYRING_DIR="${VAULT_DIR}/keyring"

# Get session ID
if [ $# -ge 1 ]; then
    SESSION_ID="$1"
else
    SESSION_ID=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null) || true
fi

if [ -z "$SESSION_ID" ]; then
    echo '{"error": "no session specified or active session not found"}'
    exit 1
fi

# Check session exists
META_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.json"
LOG_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.log"

if [ ! -f "$META_FILE" ]; then
    echo '{"error": "session not found", "session_id": "'"$SESSION_ID"'"}'
    exit 1
fi

# Revoke session
echo "revoked" > "$META_FILE"
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) | revoke | session | revoked | user" >> "$LOG_FILE"

# Remove from active keyring
if [ "$SESSION_ID" = "$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null)" ]; then
    rm -f "${KEYRING_DIR}/active_session.txt"
fi

# Commit revocation
if git rev-parse --git-dir >/dev/null 2>&1; then
    git add "$META_FILE" "$LOG_FILE" "${KEYRING_DIR}/active_session.txt" 2>/dev/null || true
    git diff --cached --quiet || git commit -m "chore(secrets-bridge): revoke session ${SESSION_ID}" 2>/dev/null || true
fi

echo '{"success": true, "action": "session_revoked", "session_id": "'"$SESSION_ID"'"}'
