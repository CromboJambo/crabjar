#!/usr/bin/env bash
# bridge-audit.sh — Show audit log and detect changes
# Usage: bridge-audit.sh [session_id]
#
# Shows recent secret access events and detects changes.
# The audit log is an immutable record, not a decision point.
# The agent reads it to understand state changes, not to ask permission.
#
# The "are you sure?" gate is theater — it gives the illusion of control
# while the operator is still the leak. Authorization comes from cryptographic
# proofs, not human gates.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
SECRETS_DIR="${VAULT_DIR}/secrets"
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

LOG_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.log"
META_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.json"

if [ ! -f "$LOG_FILE" ]; then
    echo '{"error": "audit log not found", "session_id": "'"$SESSION_ID"'"}'
    exit 1
fi

# Show audit log
echo "=== Audit Log — ${SESSION_ID} ==="
echo "Last modified: $(stat -c %y "$LOG_FILE" 2>/dev/null || stat -f %Sm "$LOG_FILE" 2>/dev/null)"
echo ""
tail -20 "$LOG_FILE"
echo ""

# Check for recent changes (last 24 hours)
RECENT=$(find "$SECRETS_DIR" -name "*.age" -mmin -1440 2>/dev/null | wc -l)
if [ "$RECENT" -gt 0 ]; then
    echo "=== Recent Changes Detected ==="
    echo "Secrets modified in last 24 hours:"
    find "$SECRETS_DIR" -name "*.age" -mmin -1440 -exec basename {} .age \; 2>/dev/null
    echo ""
    
    # Output as JSON for agent consumption
    # The agent reads this to understand state, not to get permission
    CHANGES=$(find "$SECRETS_DIR" -name "*.age" -mmin -1440 -exec basename {} .age \; 2>/dev/null | tr '\n' ',' | sed 's/,$//')
    echo '{"success": true, "action": "changes_detected", "session_id": "'"$SESSION_ID"'", "changes": "'"$CHANGES"'", "count": '"$RECENT"'}'
else
    echo '{"success": true, "action": "audit_complete", "session_id": "'"$SESSION_ID"'", "recent_changes": 0}'
fi
