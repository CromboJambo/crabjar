#!/usr/bin/env bash
# bridge-init.sh — Initialize secrets vault and generate first keypair
# Usage: bridge-init.sh
# Creates vault/ directory structure and generates X25519 keypair

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
SECRETS_DIR="${VAULT_DIR}/secrets"
KEYRING_DIR="${VAULT_DIR}/keyring"

# Check if vault already exists
if [ -d "$VAULT_DIR" ]; then
    echo '{"error": "vault already exists", "vault_dir": "'"$VAULT_DIR"'"}'
    exit 1
fi

# Create directory structure
mkdir -p "$KEYPAIRS_DIR" "$SECRETS_DIR" "$KEYRING_DIR"

# Create vault .gitignore
cat > "${VAULT_DIR}/.gitignore" << 'EOF'
# Secrets vault — never commit to git
secrets/
keypairs/*.priv
keypairs/*.json
keyring/
*.age
EOF

# Generate keypair using age-keygen (age-compatible format)
SESSION_ID="session-$(openssl rand -hex 6)"
PRIV_KEY="${KEYPAIRS_DIR}/${SESSION_ID}.priv"
PUB_KEY="${KEYPAIRS_DIR}/${SESSION_ID}.pub"
LOG_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.log"
META_FILE="${KEYPAIRS_DIR}/${SESSION_ID}.json"

# Generate private key
age-keygen -o "$PRIV_KEY" 2>/dev/null

# Extract public key
age-keygen -y "$PRIV_KEY" 2>/dev/null | grep "^age1" > "$PUB_KEY"

# Get public key hash for tracking
PUB_HASH=$(cat "$PUB_KEY" | openssl dgst -sha256 | awk '{print $NF}')

# Write metadata (no private key)
cat > "$META_FILE" << EOF
{
  "session_id": "${SESSION_ID}",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "expires_at": "$(date -u -d '+4 hours' +%Y-%m-%dT%H:%M:%SZ)",
  "public_key_hash": "${PUB_HASH}",
  "status": "active"
}
EOF

# Write active session
echo "$SESSION_ID" > "${KEYRING_DIR}/active_session.txt"

# Initialize audit log
cat > "$LOG_FILE" << EOF
# Audit Log — ${SESSION_ID}
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
# Public Key Hash: ${PUB_HASH}
#
# Format: TIMESTAMP | ACTION | SECRET | HASH | STATUS
EOF

# Commit public key to git (if in a repo)
if git rev-parse --git-dir >/dev/null 2>&1; then
    git add "${PUB_KEY}" "${LOG_FILE}" "$META_FILE" "${KEYRING_DIR}/active_session.txt" 2>/dev/null || true
    git diff --cached --quiet || git commit -m "chore(secrets-bridge): add session keypair ${SESSION_ID}" 2>/dev/null || true
fi

# Output result as JSON
cat << EOF
{
  "success": true,
  "session_id": "${SESSION_ID}",
  "public_key": "$(cat "$PUB_KEY")",
  "public_key_hash": "${PUB_HASH}",
  "vault_dir": "${VAULT_DIR}",
  "expires_at": "$(date -u -d '+4 hours' +%Y-%m-%dT%H:%M:%SZ)",
  "note": "Private key stored at ${PRIV_KEY} — never commit to git"
}
EOF
