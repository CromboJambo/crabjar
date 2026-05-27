#!/usr/bin/env bash
# bridge-keyring.sh — Manage active sessions and key rotation
# Usage: bridge-keyring.sh [--rotate|--status|--list]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYRING_DIR="${VAULT_DIR}/keyring"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[-]${NC} $*" >&2; exit 1; }

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check vault exists
if [ ! -d "$VAULT_DIR" ]; then
    error "vault not found — run bridge-init.sh first"
fi

case "${1:---status}" in
    --status)
        ACTIVE_SESSION=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null) || true
        if [ -z "$ACTIVE_SESSION" ]; then
            echo '{"error": "no active session"}'
            exit 1
        fi
        META_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.json"
        if [ ! -f "$META_FILE" ]; then
            echo '{"error": "session metadata not found", "session_id": "'"$ACTIVE_SESSION"'"}'
            exit 1
        fi
        EXPIRES=$(jq -r '.expires_at' "$META_FILE")
        EXPIRES_TS=$(date -d "$EXPIRES" +%s 2>/dev/null)
        NOW_TS=$(date +%s)
        REMAINING=$(( (EXPIRES_TS - NOW_TS) / 3600 ))
        
        echo '{"active_session": "'"$ACTIVE_SESSION"'", "expires_at": "'"$EXPIRES"'", "remaining_hours": '"$REMAINING"'}'
        ;;
    
    --rotate)
        info "Rotating session keys"
        
        # Save old session
        OLD_SESSION=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null) || true
        if [ -n "$OLD_SESSION" ]; then
            warn "Old session: $OLD_SESSION"
        fi
        
        # Generate new keypair
        SESSION_ID="session-$(openssl rand -hex 6)"
        PRIV_KEY="${KEYPAIRS_DIR}/${SESSION_ID}.priv"
        PUB_KEY="${KEYPAIRS_DIR}/${SESSION_ID}.pub"
        LOG_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.log"
        META_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.json"
        
        age-keygen -o "$PRIV_KEY" 2>/dev/null
        age-keygen -y "$PRIV_KEY" 2>/dev/null | grep "^age1" > "$PUB_KEY"
        
        PUB_HASH=$(cat "$PUB_KEY" | openssl dgst -sha256 | awk '{print $NF}')
        
        cat > "$META_FILE" << EOF
{
  "session_id": "${SESSION_ID}",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "expires_at": "$(date -u -d '+4 hours' +%Y-%m-%dT%H:%M:%SZ)",
  "public_key_hash": "${PUB_HASH}",
  "status": "active"
}
EOF
        
        echo "$SESSION_ID" > "${KEYRING_DIR}/active_session.txt"
        
        # Log rotation
        cat > "$LOG_FILE" << EOF
# Audit Log — ${SESSION_ID}
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
# Public Key Hash: ${PUB_HASH}
# Rotated from: ${OLD_SESSION:-none}
#
# Format: TIMESTAMP | ACTION | SECRET | HASH | STATUS
EOF
        
        # Commit to git
        if git rev-parse --git-dir >/dev/null 2>&1; then
            git add "${PUB_KEY}" "$META_FILE" "${KEYRING_DIR}/active_session.txt" "$LOG_FILE" 2>/dev/null || true
            git diff --cached --quiet || git commit -m "chore(secrets-bridge): rotate session to ${SESSION_ID}" 2>/dev/null || true
        fi
        
        info "Session rotated to: $SESSION_ID"
        echo '{"success": true, "session_id": "'"$SESSION_ID"'", "note": "Re-encrypt secrets with new public key"}'
        ;;
    
    --list)
        echo '{"sessions": ['
        FIRST=true
        for META in "${KEYPAIRS_DIR}"/*.json; do
            [ -f "$META" ] || continue
            SESSION_ID=$(jq -r '.session_id' "$META")
            STATUS=$(jq -r '.status' "$META")
            EXPIRES=$(jq -r '.expires_at' "$META")
            if [ "$FIRST" = true ]; then
                FIRST=false
            else
                echo ","
            fi
            echo "  {\"session_id\": \"${SESSION_ID}\", \"status\": \"${STATUS}\", \"expires_at\": \"${EXPIRES}\"}"
        done
        echo "]}"
        ;;
    
    --help)
        echo "Usage: bridge-keyring.sh [--status|--rotate|--list]"
        echo ""
        echo "  --status  Show active session info"
        echo "  --rotate  Generate new keypair (rotate keys)"
        echo "  --list    List all sessions"
        ;;
    
    *)
        error "Unknown option: $1 (use --help)"
        ;;
esac
