#!/usr/bin/env bash
# bridge-rotate.sh — Remind user to rotate secrets and rotate session keys
# Usage: bridge-rotate.sh [--check|--rotate|--remind]
#
# The vault is not a replacement for rotation. It's a temporary holding place.
# The real security comes from rotating the original secrets and removing them
# from their original location.
#
# This script:
# 1. Checks if rotation is needed
# 2. Reminds the user to rotate original secrets
# 3. Rotates session keys if confirmed
# 4. Logs the rotation event

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
SECRETS_DIR="${VAULT_DIR}/secrets"
KEYRING_DIR="${VAULT_DIR}/keyring"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[-]${NC} $*" >&2; exit 1; }

# Check vault exists
if [ ! -d "$VAULT_DIR" ]; then
    error "vault not found — run bridge-init.sh first"
fi

# Get active session
ACTIVE_SESSION=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null) || true
if [ -z "$ACTIVE_SESSION" ]; then
    error "no active session"
fi

META_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.json"
LOG_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.log"

# Check session expiry
EXPIRES=$(jq -r '.expires_at' "$META_FILE")
EXPIRES_TS=$(date -d "$EXPIRES" +%s 2>/dev/null)
NOW_TS=$(date +%s)
REMAINING=$(( (EXPIRES_TS - NOW_TS) / 3600 ))

case "${1:---remind}" in
    --check)
        # Check if rotation is needed
        if [ "$REMAINING" -le 1 ]; then
            echo '{"action": "rotation_needed", "remaining_hours": '"$REMAINING"'}'
            exit 0
        else
            echo '{"action": "rotation_not_needed", "remaining_hours": '"$REMAINING"'}'
            exit 0
        fi
        ;;
    
    --rotate)
        # Rotate session keys
        info "Rotating session keys"
        
        # Save old session
        OLD_SESSION="$ACTIVE_SESSION"
        
        # Generate new keypair
        NEW_SESSION_ID="session-$(openssl rand -hex 6)"
        PRIV_KEY="${KEYPAIRS_DIR}/${NEW_SESSION_ID}.priv"
        PUB_KEY="${KEYPAIRS_DIR}/${NEW_SESSION_ID}.pub"
        LOG_FILE="${KEYPAIRS_DIR}/${NEW_SESSION_ID}.log"
        META_FILE="${KEYPAIRS_DIR}/${NEW_SESSION_ID}.json"
        
        age-keygen -o "$PRIV_KEY" 2>/dev/null
        age-keygen -y "$PRIV_KEY" 2>/dev/null | grep "^age1" > "$PUB_KEY"
        
        PUB_HASH=$(cat "$PUB_KEY" | openssl dgst -sha256 | awk '{print $NF}')
        
        cat > "$META_FILE" << EOF
{
  "session_id": "${NEW_SESSION_ID}",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "expires_at": "$(date -u -d '+4 hours' +%Y-%m-%dT%H:%M:%SZ)",
  "public_key_hash": "${PUB_HASH}",
  "status": "active"
}
EOF
        
        echo "$NEW_SESSION_ID" > "${KEYRING_DIR}/active_session.txt"
        
        # Log rotation
        cat > "$LOG_FILE" << EOF
# Audit Log — ${NEW_SESSION_ID}
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
# Public Key Hash: ${PUB_HASH}
# Rotated from: ${OLD_SESSION}
# Reason: Scheduled rotation
#
# Format: TIMESTAMP | ACTION | SECRET | HASH | STATUS
EOF
        
        # Commit to git
        if git rev-parse --git-dir >/dev/null 2>&1; then
            git add "${PUB_KEY}" "$META_FILE" "${KEYRING_DIR}/active_session.txt" "$LOG_FILE" 2>/dev/null || true
            git diff --cached --quiet || git commit -m "chore(secrets-bridge): rotate session to ${NEW_SESSION_ID}" 2>/dev/null || true
        fi
        
        info "Session rotated to: $NEW_SESSION_ID"
        echo '{"success": true, "session_id": "'"$NEW_SESSION_ID"'", "note": "Re-encrypt secrets with new public key"}'
        ;;
    
    --remind)
        # Remind user to rotate
        echo ""
        info "=== Rotation Reminder ==="
        info "Session: $ACTIVE_SESSION"
        info "Expires: $EXPIRES"
        info "Remaining: ${REMAINING} hours"
        echo ""
        
        if [ "$REMAINING" -le 1 ]; then
            warn "⚠ Session expires soon!"
            warn "Run 'bridge-rotate.sh --rotate' to rotate session keys"
        else
            info "Session is valid for ${REMAINING} more hours"
        fi
        
        echo ""
        warn "=== Important ==="
        warn "The vault is NOT a replacement for rotation."
        warn "The act of adding secrets to the vault is itself a leak event."
        warn "The password manager, terminal, and clipboard are all potential leak points."
        echo ""
        warn "Remember to:"
        warn "1. Rotate all original secrets"
        warn "2. Remove them from their original locations"
        warn "3. Run 'bridge-rotate.sh --rotate' to rotate session keys"
        echo ""
        ;;
    
    --help)
        echo "Usage: bridge-rotate.sh [--check|--rotate|--remind]"
        echo ""
        echo "  --check   Check if rotation is needed"
        echo "  --rotate  Rotate session keys"
        echo "  --remind  Remind user to rotate"
        ;;
    
    *)
        error "Unknown option: $1 (use --help)"
        ;;
esac
