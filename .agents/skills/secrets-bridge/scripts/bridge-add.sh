#!/usr/bin/env bash
# bridge-add.sh — Add secrets to vault with explicit rotation loop
# Usage: bridge-add.sh
#
# This script implements the "write → test → confirm → rotate" loop.
# The agent "looks the other way" while the user applies secrets.
# The user is responsible for rotating the original secrets after this completes.
#
# The vault is not a replacement for rotation. It's a temporary holding place.
# The real security comes from rotating the original secrets and removing them
# from their original location.

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

PUB_KEY=$(cat "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.pub")
info "Using public key: ${PUB_KEY:0:20}..."

# List current secrets
echo ""
info "Current secrets in vault:"
find "$SECRETS_DIR" -name "*.age" 2>/dev/null | while read -r f; do
    echo "  - $(basename "$f" .age)"
done
echo ""

# Interactive secret addition loop
while true; do
    # Prompt for secret name
    read -p "Enter secret name (or 'done' to finish): " NAME
    
    if [[ "$NAME" == "done" ]] || [[ -z "$NAME" ]]; then
        break
    fi
    
    # Check if secret already exists
    if [ -f "${SECRETS_DIR}/${NAME}.age" ]; then
        warn "Secret '$NAME' already exists in vault"
        read -p "Overwrite? [y/N]: " CONFIRM
        if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
            info "Skipping '$NAME'"
            continue
        fi
    fi
    
    # Encrypt secret (user pastes input, nothing stored in shell)
    info "Encrypting '$NAME'..."
    age -r "$PUB_KEY" -o "${SECRETS_DIR}/${NAME}.age" 2>/dev/null
    
    # Verify encryption worked
    if age -d -i "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.priv" "${SECRETS_DIR}/${NAME}.age" 2>/dev/null; then
        info "✓ '$NAME' encrypted and verified"
    else
        error "✗ Encryption failed — secret not added"
        rm -f "${SECRETS_DIR}/${NAME}.age"
        continue
    fi
    
    # Prompt for rotation (the critical security step)
    echo ""
    warn "IMPORTANT: The vault is NOT a replacement for rotation."
    warn "The act of adding this secret to the vault is itself a leak event."
    warn "The password manager, terminal, and clipboard are all potential leak points."
    echo ""
    read -p "Have you rotated the original secret and removed it from its original location? [y/N]: " ROTATE_CONFIRM
    
    if [[ ! "$ROTATE_CONFIRM" =~ ^[Yy]$ ]]; then
        warn "⚠ Rotation not confirmed."
        warn "The original secret still exists in its original location."
        warn "This is a security risk."
    else
        info "✓ Rotation confirmed"
    fi
    
    echo ""
done

# Final reminder
echo ""
info "=== Summary ==="
info "Vault now contains: $(find "$SECRETS_DIR" -name "*.age" 2>/dev/null | wc -l) secrets"
info "Session expires: $(jq -r '.expires_at' "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.json")"
echo ""
warn "Remember: rotate all original secrets and remove them from their original locations."
warn "The vault is a temporary holding place, not a permanent solution."
warn "Run 'bridge-keyring.sh --rotate' to rotate session keys."
echo ""
