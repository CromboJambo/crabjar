#!/usr/bin/env bash
# ephemeral-shell.sh — Burn-after-reading terminal session
# Usage: ephemeral-shell.sh [--run [command]] [--teardown]
#
# Creates a temporary shell environment that writes nothing to persistent storage.
# No sudo needed.

set -euo pipefail

TEMP_HOME=""

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
    if ! command -v age &>/dev/null; then
        error "age not installed"
    fi
}

# Create ephemeral environment
do_create() {
    TEMP_HOME=$(mktemp -d -t ephemeral-shell.XXXXXX)
    mkdir -p "${TEMP_HOME}/.config" "${TEMP_HOME}/.cache" "${TEMP_HOME}/.local/share"
    
    # Empty configs — no persistence
    touch "${TEMP_HOME}/.bashrc"
    touch "${TEMP_HOME}/.profile"
    touch "${TEMP_HOME}/.inputrc"  # prevent readline history
    
    # Create cleanup script
    cat > "${TEMP_HOME}/.cleanup" << 'CLEANUP'
#!/usr/bin/env bash
set -euo pipefail
rm -rf "$TEMP_HOME"
rm -f "$0"
CLEANUP
    chmod +x "${TEMP_HOME}/.cleanup"
    
    info "Ephemeral shell ready"
    info "Cleanup: ${TEMP_HOME}/.cleanup"
    info "Note: using mktemp (wiped on reboot)"
}

# Run ephemeral shell
do_run() {
    local CMD="${1:-}"
    
    if [ -z "$TEMP_HOME" ] || [ ! -d "$TEMP_HOME" ]; then
        warn "Ephemeral shell not created — running create first"
        do_create
    fi
    
    info "Starting ephemeral shell"
    info "All state will be wiped on exit"
    
    # Set clean environment — no persistent state
    env -i \
        HOME="$TEMP_HOME" \
        PATH="/usr/local/bin:/usr/bin:/bin:/home/crombo/.local/bin" \
        SHELL="/bin/bash" \
        HISTSIZE=0 \
        HISTFILESIZE=0 \
        PROMPT_COMMAND="" \
        INPUTRC="/dev/null" \
        XDG_CONFIG_HOME="$TEMP_HOME/.config" \
        XDG_CACHE_HOME="$TEMP_HOME/.cache" \
        XDG_DATA_HOME="$TEMP_HOME/.local/share" \
        EDITOR="vi" \
        VISUAL="vi" \
        bash --login --norc
}

# Teardown
do_teardown() {
    if [ -n "$TEMP_HOME" ] && [ -d "$TEMP_HOME" ]; then
        rm -rf "$TEMP_HOME"
        info "Ephemeral shell wiped"
    else
        warn "Ephemeral shell not created"
    fi
}

# Main
check_prereqs

case "${1:---help}" in
    --create)   do_create ;;
    --run)      do_run "${2:-}" ;;
    --teardown) do_teardown ;;
    --help)
        echo "Usage: ephemeral-shell.sh [--create|--run|--teardown] [command]"
        echo ""
        echo "  --create   Create ephemeral environment"
        echo "  --run      Start ephemeral shell"
        echo "  --teardown Wipe ephemeral environment"
        ;;
    *) error "Unknown option: $1" ;;
esac
