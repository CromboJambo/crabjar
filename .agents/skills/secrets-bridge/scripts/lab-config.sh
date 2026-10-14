#!/usr/bin/env bash
# lab-config.sh — Store and inject home-lab configuration via secrets bridge
#
# Usage:
#   lab-config.sh store <key> <value>     Store encrypted config value
#   lab-config.sh get <key>               Retrieve decrypted config value
#   lab-config.sh list                    List stored config keys
#   lab-config.sh inject                  Export all config as env vars
#   lab-config.sh exec <command...>       Run command with injected config
#
# Config values are encrypted with AES-256-CBC using a vault master key.
# The master key is derived from the active session's private key file.

set -euo pipefail

VAULT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../vault" && pwd)"
SECRETS_DIR="${VAULT_DIR}/secrets"
KEYPAIRS_DIR="${VAULT_DIR}/keypairs"
KEYRING_FILE="${VAULT_DIR}/keyring/active_session.txt"

info()  { echo "[+] $*"; }
error() { echo "[-] $*" >&2; exit 1; }

# Get active session name
get_active_session() {
    if [ ! -f "$KEYRING_FILE" ]; then
        error "No active session. Run bridge-init.sh first."
    fi
    cat "$KEYRING_FILE"
}

# Derive encryption key from session private key (use file path as passphrase)
get_encryption_key() {
    local session=$(get_active_session)
    local priv_file="${KEYPAIRS_DIR}/${session}.priv"
    
    if [ ! -f "$priv_file" ]; then
        error "Private key not found for session ${session}"
    fi
    
    echo "$priv_file"
}

# Check vault is initialized
check_vault() {
    if [ ! -d "$SECRETS_DIR" ]; then
        error "Vault not initialized. Run bridge-init.sh first."
    fi
    get_active_session >/dev/null 2>&1 || error "No active session in vault"
}

# Encrypt and store config value
store() {
    local key="$1"
    local value="$2"
    
    check_vault
    
    local priv_file=$(get_encryption_key)
    
    # Use openssl with the private key file as passphrase source
    echo -n "$value" | openssl enc -aes-256-cbc -salt -pbkdf2 \
        -pass file:"$priv_file" \
        -out "${SECRETS_DIR}/lab-${key}.enc" 2>/dev/null
    
    info "Stored encrypted config: lab-${key}"
}

# Decrypt and return config value
get() {
    local key="$1"
    
    check_vault
    
    local file="${SECRETS_DIR}/lab-${key}.enc"
    if [ ! -f "$file" ]; then
        error "Config not found: lab-${key}"
    fi
    
    local priv_file=$(get_encryption_key)
    
    openssl enc -d -aes-256-cbc -salt -pbkdf2 \
        -pass file:"$priv_file" \
        -in "$file" 2>/dev/null
}

# Export all lab config as environment variables
inject() {
    check_vault
    
    local session=$(get_active_session)
    
    info "Injecting lab configuration from vault (session: ${session})..."
    
    local priv_file=$(get_encryption_key)
    
    # Find all lab-* encrypted files
    for file in "${SECRETS_DIR}"/lab-*.enc; do
        [ -f "$file" ] || continue
        
        # Extract key name (remove lab- prefix and .enc suffix)
        local basename=$(basename "$file" .enc)
        local key="${basename#lab-}"
        
        # Decrypt and export
        local value=$(openssl enc -d -aes-256-cbc -salt -pbkdf2 \
            -pass file:"$priv_file" \
            -in "$file" 2>/dev/null)
        
        if [ -n "$value" ]; then
            export "${key}=${value}"
            info "  Exported: ${key}"
        fi
    done
    
    return 0
}

# List stored config keys
list() {
    check_vault
    
    local found=false
    for file in "${SECRETS_DIR}"/lab-*.enc; do
        [ -f "$file" ] || continue
        local basename=$(basename "$file" .enc)
        local key="${basename#lab-}"
        echo "  ${key}"
        found=true
    done
    
    if [ "$found" = false ]; then
        info "No lab configuration stored yet."
        info "Store values with: $0 store <KEY> <value>"
    fi
}

# Run command with injected config
exec_with_config() {
    inject
    exec "$@"
}

# Main
case "${1:-}" in
    store)
        if [ $# -lt 3 ]; then
            error "Usage: $0 store <key> <value>"
        fi
        store "$2" "$3"
        ;;
    get)
        if [ $# -lt 2 ]; then
            error "Usage: $0 get <key>"
        fi
        get "$2"
        ;;
    inject)
        inject
        ;;
    list)
        list
        ;;
    exec)
        shift
        exec_with_config "$@"
        ;;
    *)
        echo "Usage: $0 {store|get|inject|list|exec} [args...]"
        echo ""
        echo "Commands:"
        echo "  store <key> <value>   Store encrypted config value"
        echo "  get <key>             Retrieve decrypted config value"
        echo "  inject                Export all config as env vars (for shell)"
        echo "  list                  List stored config keys"
        echo "  exec <cmd...>         Run command with injected config"
        exit 1
        ;;
esac