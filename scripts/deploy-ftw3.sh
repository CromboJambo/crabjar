#!/usr/bin/env bash
# Deploy crabjar to ftw3 home-lab node.
# Usage: ./scripts/deploy-ftw3.sh [commit-ish]
#   Default commit: current HEAD
# What it does:
#   1. Builds release binary locally (Linux x86_64)
#   2. SSH to ftw3, stop crabjar service if running
#   3. rsync the built artifact to /opt/crabjar/ on ftw3
#   4. Restart crabjar service on ftw3

set -euo pipefail

COMMIT="${1:-HEAD}"
SSH_KEY="$HOME/.ssh/id_lab"
DEPLOY_DIR="/opt/crabjar"
REMOTE_USER="crombo"
REMOTE_HOST="ftw3"

echo "==> Building release binary for commit $COMMIT..."
cargo build --release -p crabjar 2>&1 | tail -5

echo "==> SSH to ftw3, stopping crabjar service (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "dinitctl stop crabjar-agent || true"

echo "==> rsync built artifact to ftw3: ${DEPLOY_DIR}/..."
rsync -avz --delete \
    target/release/crabjar \
    "${REMOTE_USER}@${REMOTE_HOST}:${DEPLOY_DIR}/crabjar"

echo "==> Restarting crabjar service on ftw3 (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "dinitctl start crabjar-agent && sleep 2 && dinitctl status crabjar-agent || true"

echo "==> Deployment complete. Commit: $COMMIT"
