#!/usr/bin/env bash
# bridge-verify.sh — Validate session and output proof of secret access
# Usage: bridge-verify.sh <secret_name>
#
# Outputs proof file (not the secret) with hash and timestamp.
# Agent never sees raw secret value.
#
# Threat model:
# - Scraper infrastructure indexes private repos programmatically
# - Agent sessions that hold session tokens can be exfiltrated
# - Human confirmation is another channel that can be compromised
# - Authorization comes from cryptographic proofs, not human gates
#
# The agent should understand WHY each step matters, not just trust the script.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
VAULT_DIR="${SKILL_DIR}/vault"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
SECRETS_DIR="${VAULT_DIR}/secrets"
KEYRING_DIR="${VAULT_DIR}/keyring"

# --- Step 1: Validate input ---
# The agent needs to understand this: missing input means the request is invalid,
# not a security failure. It's a structural check, not a gate.
if [ $# -lt 1 ]; then
    echo '{"error": "missing secret_name", "usage": "bridge-verify.sh <secret_name>"}'
    exit 1
fi

SECRET_NAME="$1"

# --- Step 2: Verify vault exists ---
# If vault doesn't exist, the agent should know: no vault = no secrets accessible.
# This is a configuration state, not a security violation.
if [ ! -d "$VAULT_DIR" ]; then
    echo '{"error": "vault not found", "hint": "Run bridge-init.sh first"}'
    exit 1
fi

# --- Step 3: Get active session ---
# The active session pointer is the agent's identity. Without it, it has no authorization.
ACTIVE_SESSION=$(cat "${KEYRING_DIR}/active_session.txt" 2>/dev/null)
if [ -z "$ACTIVE_SESSION" ]; then
    echo '{"error": "no active session", "hint": "Run bridge-init.sh first"}'
    exit 1
fi

# --- Step 4: Validate session identity ---
# The public key is the session's identity. If it doesn't exist, the session is invalid.
PUB_KEY="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.pub"
META_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.json"
LOG_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}.log"

if [ ! -f "$PUB_KEY" ]; then
    echo '{"error": "session not found", "session_id": "'"$ACTIVE_SESSION"'"}'
    exit 1
fi

# --- Step 5: Check session expiry ---
# Expiry is not a gate — it's a state change. Expired sessions can't be extended,
# they must be regenerated. This is by design: short-lived access limits exposure.
EXPIRES=$(jq -r '.expires_at' "$META_FILE" 2>/dev/null)
if [ -n "$EXPIRES" ] && [ "$(date -d "$EXPIRES" +%s 2>/dev/null)" -lt "$(date +%s)" ]; then
    echo '{"error": "session expired", "session_id": "'"$ACTIVE_SESSION"'", "hint": "Run bridge-init.sh to generate new keypair"}'
    exit 1
fi

# --- Step 6: Verify secret is registered ---
# The secret must be in the vault for this session. This is a namespace check,
# not a gate. If it's not registered, the agent should know it doesn't exist.
SECRET_FILE="${SECRETS_DIR}/${SECRET_NAME}.age"
if [ ! -f "$SECRET_FILE" ]; then
    echo '{"error": "secret not registered", "secret_name": "'"$SECRET_NAME"'", "hint": "Add secret to vault/secrets/ first"}'
    exit 2
fi

# --- Step 7: Decrypt (in memory only) ---
# This is the only step where the raw secret is touched. It happens in memory,
# never written to disk. The agent never sees it — only the hash.
DECRYPTED=$(age -d -i "${KEYPAIRS_DIR}/${ACTIVE_SESSION}.priv" "$SECRET_FILE" 2>/dev/null) || {
    echo '{"error": "decryption failed", "hint": "Check private key or secret file"}'
    exit 4
}

# --- Step 8: Generate proof (not the secret) ---
# The proof is what the agent receives. It contains:
# - The hash of the secret (for integrity verification)
# - The timestamp (for audit)
# - The session ID (for attribution)
# - The public key hash (for identity verification)
#
# The agent can verify the proof is valid without seeing the secret.
PROOF_HASH=$(echo "$DECRYPTED" | openssl dgst -sha256 | awk '{print $NF}')
PROOF_FILE="${KEYPAIRS_DIR}/${ACTIVE_SESSION}-proof-$(date +%s).json"
PROOF_TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cat > "$PROOF_FILE" << EOF
{
  "secret": "${SECRET_NAME}",
  "used_at": "${PROOF_TIMESTAMP}",
  "hash": "${PROOF_HASH}",
  "session_id": "${ACTIVE_SESSION}",
  "public_key_hash": "$(jq -r '.public_key_hash' "$META_FILE")",
  "verified": true
}
EOF

# --- Step 9: Log access (append-only) ---
# The audit log is an immutable record, not a decision point.
# The agent reads it to understand state changes, not to ask permission.
echo "${PROOF_TIMESTAMP} | verify | ${SECRET_NAME} | ${PROOF_HASH:0:16}... | success" >> "$LOG_FILE"

# --- Output: Proof only, never the secret ---
cat << EOF
{
  "success": true,
  "secret": "${SECRET_NAME}",
  "proof_file": "${PROOF_FILE}",
  "hash": "${PROOF_HASH}",
  "used_at": "${PROOF_TIMESTAMP}",
  "session_id": "${ACTIVE_SESSION}",
  "note": "Proof file contains hash of secret value — raw secret was not output"
}
EOF
