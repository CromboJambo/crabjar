#!/usr/bin/env bash
# Deploy vm-bridge to ftw3 home-lab node.
# Follows the same pattern as pesti-server deployment.
#
# Usage: ./scripts/deploy-vm-bridge-ftw3.sh [commit-ish]
#   Default commit: current HEAD
#
# What it does:
#   1. Builds release binary locally (Linux x86_64)
#   2. SSH to ftw3, stop vm-bridge service if running
#   3. rsync the built artifact to /opt/crabjar/vm-bridge on ftw3
#   4. Restart vm-bridge service on ftw3 (dinit)

set -euo pipefail

COMMIT="${1:-HEAD}"
SSH_KEY="$HOME/.ssh/id_lab"
DEPLOY_DIR="/opt/crabjar/vm-bridge"
REMOTE_USER="crombo"
REMOTE_HOST="ftw3"

echo "==> Building vm-bridge release binary for commit $COMMIT..."
cargo build --release -p vm-bridge 2>&1 | tail -5

echo "==> SSH to ftw3, stopping vm-bridge service (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "sudo dinitctl stop vm-bridge || true"

echo "==> rsync built artifact to ftw3: ${DEPLOY_DIR}/..."
rsync -avz --delete \
    target/release/vm-bridge \
    "${REMOTE_USER}@${REMOTE_HOST}:${DEPLOY_DIR}/vm-bridge"

echo "==> Restarting vm-bridge service on ftw3 (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "sudo dinitctl start vm-bridge && sleep 2 && ps aux | grep vm-bridge | head -5"

echo "==> Deployment complete. Commit: $COMMIT"
echo "==> Lifecycle API available at http://ftw3:8090/vms (via SSH tunnel)"
