#!/usr/bin/env bash
# bridge-discover.sh — Discover secrets via pattern matching without exposing values
# Usage: bridge-discover.sh [pattern]
# Lists secret names and metadata matching pattern
# Uses fzf for interactive selection, rg for pattern matching

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
SECRETS_DIR="${VAULT_DIR}/secrets"
KEYRING_DIR="${VAULT_DIR}/keyring"

# Check vault exists
if [ ! -d "$VAULT_DIR" ]; then
    echo '{"error": "vault not found", "hint": "Run bridge-init.sh first"}'
    exit 1
fi

# Check for secrets
SECRET_FILES=$(find "$SECRETS_DIR" -name "*.age" 2>/dev/null)
if [ -z "$SECRET_FILES" ]; then
    echo '{"success": true, "secrets": [], "note": "no secrets registered"}'
    exit 0
fi

# Get active session
ACTIVE_SESSION=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null) || true

# Build secret list with metadata
SECRET_LIST=""
while IFS= read -r SECRET_FILE; do
    SECRET_NAME=$(basename "$SECRET_FILE" .age)
    
    # Get hash prefix (first 8 chars of sha256)
    HASH_PREFIX=$(age -d -i "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.priv" "$SECRET_FILE" 2>/dev/null | openssl dgst -sha256 | awk '{print substr($NF, 1, 8)}' 2>/dev/null) || HASH_PREFIX="???"
    
    # Get last accessed from audit log
    LAST_ACCESSED=$(grep "$SECRET_NAME" "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.log" 2>/dev/null | tail -1 | awk -F'|' '{print $1}' 2>/dev/null) || LAST_ACCESSED="never"
    
    SECRET_LIST+="${SECRET_NAME}|${HASH_PREFIX}|${LAST_ACCESSED}|${SECRET_FILE}"$'\n'
done <<< "$SECRET_FILES"

# Remove trailing newline
SECRET_LIST=$(echo "$SECRET_LIST" | sed '/^$/d')

# Apply pattern filter if provided
if [ $# -ge 1 ]; then
    PATTERN="$1"
    SECRET_LIST=$(echo "$SECRET_LIST" | rg "$PATTERN" 2>/dev/null) || SECRET_LIST=""
fi

# Interactive selection with fzf if no pattern and interactive terminal
if [ -t 0 ] && [ -z "$PATTERN" ]; then
    SELECTED=$(echo "$SECRET_LIST" | fzf --delimiter='|' --format='plain' --preview="echo 'Hash: {{3}}' | head -1" 2>/dev/null) || true
    if [ -n "$SELECTED" ]; then
        SECRET_NAME=$(echo "$SELECTED" | cut -d'|' -f1)
        HASH_PREFIX=$(echo "$SELECTED" | cut -d'|' -f2)
        echo '{"success": true, "selected": "'"$SECRET_NAME"'", "hash_prefix": "'"$HASH_PREFIX"'"}'
    else
        echo '{"success": true, "action": "selection_cancelled"}'
    fi
else
    # Output list as JSON
    echo '{"success": true, "secrets": ['
    FIRST=true
    while IFS='|' read -r NAME HASH LAST FILE; do
        if [ "$FIRST" = true ]; then
            FIRST=false
        else
            echo ","
        fi
        echo "  {\"name\": \"${NAME}\", \"hash_prefix\": \"${HASH}\", \"last_accessed\": \"${LAST}\"}"
    done <<< "$SECRET_LIST"
    echo "]"
fi
