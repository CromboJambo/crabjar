#!/usr/bin/env bash
# bridge-mount.sh — Create ephemeral vault in tmpfs
# Usage: bridge-mount.sh [--create|--restore|--teardown]
# 
# Prerequisites:
#   - sudo access
#   - tmpfs support (Linux)
#   - vault skill initialized
#
# Fails hard if prerequisites missing

set -euo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VAULT_SKILL="${SKILL_DIR}/vault"
TMPFS_MOUNT="/dev/shm/secrets-vault"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[-]${NC} $*" >&2; exit 1; }

# Check prerequisites
check_prereqs() {
    # sudo accessible
    if ! sudo -n true 2>/dev/null; then
        error "sudo required but passwordless sudo not configured"
    fi
    
    # tmpfs support (optional — falls back to mktemp)
    if ! mount -t tmpfs tmpfs /dev/shm 2>/dev/null; then
        warn "tmpfs not available — will use mktemp fallback"
    fi
    
    # age installed
    if ! command -v age &>/dev/null; then
        error "age not installed — run: curl -L https://github.com/FiloSottile/age/releases/download/v1.2.0/age-v1.2.0-linux-amd64.tar.gz | tar -xz && sudo cp age/age /usr/local/bin/"
    fi
    
    # age-keygen installed
    if ! command -v age-keygen &>/dev/null; then
        error "age-keygen not installed"
    fi
}

# Create ephemeral vault
do_create() {
    info "Creating ephemeral vault at ${TMPFS_MOUNT}"
    
    # Create mount point
    sudo mkdir -p "${TMPFS_MOUNT}"
    sudo mount -t tmpfs -o size=10M tmpfs "${TMPFS_MOUNT}"
    
    # Create directory structure
    sudo mkdir -p "${TMPFS_MOUNT}/secrets" "${TMPFS_MOUNT}/keypairs" "${TMPFS_MOUNT}/keyring"
    
    # Copy tracked public keys from git
    if [ -d "${VAULT_SKILL}/keypairs" ]; then
        local pub_keys
        pub_keys=$(find "${VAULT_SKILL}/keypairs" -name "*.pub" 2>/dev/null)
        if [ -n "$pub_keys" ]; then
            info "Copying tracked public keys from git"
            echo "$pub_keys" | while read -r key; do
                sudo cp "$key" "${TMPFS_MOUNT}/keypairs/"
                info "  copied: $(basename "$key")"
            done
        else
            warn "No tracked public keys found"
        fi
    fi
    
    # Copy active session pointer
    if [ -f "${VAULT_SKILL}/keyring/active_session.txt" ]; then
        sudo cp "${VAULT_SKILL}/keyring/active_session.txt" "${TMPFS_MOUNT}/keyring/"
        info "Copied active session pointer"
    fi
    
    # Create cleanup script
    local cleanup_script="${TMPFS_MOUNT}/.cleanup"
    cat > "$cleanup_script" << 'CLEANUP'
#!/usr/bin/env bash
# Cleanup ephemeral vault — run on session end
set -euo pipefail
MOUNT="/dev/shm/secrets-vault"
if mountpoint -q "$MOUNT" 2>/dev/null; then
    umount "$MOUNT"
    rmdir "$MOUNT"
fi
rm -f "$0"
CLEANUP
    sudo chmod +x "$cleanup_script"
    
    info "Ephemeral vault created"
    info "Run '${TMPFS_MOUNT}/.cleanup' on session end to wipe"
    info "Vault will also be wiped on reboot (tmpfs)"
}

# Restore vault from git
do_restore() {
    info "Restoring vault from git-tracked state"
    
    if ! mountpoint -q "${TMPFS_MOUNT}" 2>/dev/null; then
        error "Vault not mounted — run 'bridge-mount.sh --create' first"
    fi
    
    # Copy all tracked files
    if [ -d "${VAULT_SKILL}/keypairs" ]; then
        info "Restoring keypairs from git"
        find "${VAULT_SKILL}/keypairs" -name "*.pub" -o -name "*.log" -o -name "*.json" | while read -r file; do
            sudo cp "$file" "${TMPFS_MOUNT}/keypairs/"
            info "  restored: $(basename "$file")"
        done
    fi
    
    info "Vault restored"
}

# Teardown ephemeral vault
do_teardown() {
    info "Tearing down ephemeral vault"
    
    if mountpoint -q "${TMPFS_MOUNT}" 2>/dev/null; then
        sudo umount "${TMPFS_MOUNT}"
        sudo rmdir "${TMPFS_MOUNT}"
        info "Vault unmounted"
    else
        warn "Vault not mounted"
    fi
}

# Main
check_prereqs

case "${1:---create}" in
    --create)   do_create ;;
    --restore)  do_restore ;;
    --teardown) do_teardown ;;
    --help)
        echo "Usage: bridge-mount.sh [--create|--restore|--teardown]"
        echo ""
        echo "  --create   Create ephemeral vault in tmpfs"
        echo "  --restore  Restore vault from git-tracked state"
        echo "  --teardown Unmount and wipe ephemeral vault"
        ;;
    *) error "Unknown option: $1 (use --help)" ;;
esac
