#!/usr/bin/env bash
# Crabjar cross-machine deployment script
# Usage: ./scripts/deploy.sh <target> [options]
# Targets: lappy, pixel9a, artix-box, ftw3, local

set -euo pipefail

TARGET="${1:-local}"
VERSION="${CRABJAR_VERSION:-$(git describe --tags --always 2>/dev/null || echo "dev")}"
ARCH=$(uname -m)
PLATFORM="linux-${ARCH}"

case "$TARGET" in
    lappy)
        HOST="crombo@lappy"
        SSH_KEY="$HOME/.ssh/id_lab"
        ;;
    pixel9a)
        HOST="crombo@100.84.247.163"
        SSH_KEY="$HOME/.ssh/id_pixel9a"
        PORT=8022
        ;;
    artix-box)
        HOST="crombo@artix-box"
        SSH_KEY="$HOME/.ssh/id_lab"
        ;;
    ftw3)
        HOST="crombo@ftw3"
        SSH_KEY="$HOME/.ssh/id_lab"
        ;;
    local)
        HOST=""
        SSH_KEY=""
        ;;
    *)
        echo "Unknown target: $TARGET"
        exit 1
        ;;
esac

echo "Deploying crabjar v${VERSION} to ${TARGET}"

# Build release binary
echo "Building release binary..."
cargo build --release -p crabjar

# Determine install path
INSTALL_DIR="/opt/crabjar"
BINARY="${INSTALL_DIR}/crabjar"
DATA_DIR="${INSTALL_DIR}/data"

if [ -n "$HOST" ]; then
    # Remote deployment via SSH
    echo "Deploying to ${HOST}..."
    
    # Create directory structure
    ssh -i "$SSH_KEY" "${HOST}" "mkdir -p ${INSTALL_DIR} ${DATA_DIR}"
    
    # Copy binary
    scp -i "$SSH_KEY" target/release/crabjar "${HOST}:${BINARY}"
    ssh -i "$SSH_KEY" "${HOST}" "chmod +x ${BINARY}"
    
    # Generate machine-specific config
    MACHINE_ID=$(ssh -i "$SSH_KEY" "${HOST}" "hostname")
    echo "Generating config for ${MACHINE_ID}..."
    
    cat > /tmp/crabjar_config.toml <<EOF
# Crabjar configuration for ${MACHINE_ID}
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

[identity]
machine_id = "${MACHINE_ID}"
version = "${VERSION}"

[guard]
trust_layer = 0
auto_grant = false

[execution]
container_runtime = "podman"  # auto-detect: podman, docker, none
sandbox_user = "crabjar-agent"
timeout_seconds = 300

[telemetry]
enabled = true
log_level = "info"
output_dir = "${DATA_DIR}/logs"
EOF
    
    scp -i "$SSH_KEY" /tmp/crabjar_config.toml "${HOST}:${INSTALL_DIR}/.crabjar_config.toml"
    
    # crabjar is a CLI tool, not a daemon - no service file needed
    echo "Deployment complete. crabjar installed to: ${INSTALL_DIR}/crabjar"
    echo "Run with: ${INSTALL_DIR}/crabjar <command>"
else
    # Local deployment
    echo "Installing locally to ${INSTALL_DIR}..."
    mkdir -p "${INSTALL_DIR}" "${DATA_DIR}"
    cp target/release/crabjar "${BINARY}"
    chmod +x "${BINARY}"
    
    MACHINE_ID=$(hostname)
    cat > "${INSTALL_DIR}/.crabjar_config.toml" <<EOF
# Crabjar configuration for ${MACHINE_ID}
# Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

[identity]
machine_id = "${MACHINE_ID}"
version = "${VERSION}"

[guard]
trust_layer = 0
auto_grant = false

[execution]
container_runtime = "podman"
sandbox_user = "crabjar-agent"
timeout_seconds = 300

[telemetry]
enabled = true
log_level = "info"
output_dir = "${DATA_DIR}/logs"
EOF
    
    echo "Local deployment complete."
fi

echo "Done!"
