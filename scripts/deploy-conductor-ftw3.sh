#!/usr/bin/env bash
# Deploy crabjar-conductor to ftw3 home-lab node.
# Follows the same pattern as pesti-server and vm-bridge deployments.
#
# Usage: ./scripts/deploy-conductor-ftw3.sh [commit-ish]
#   Default commit: current HEAD

set -euo pipefail

COMMIT="${1:-HEAD}"
SSH_KEY="$HOME/.ssh/id_lab"
DEPLOY_DIR="/opt/crabjar/conductor"
REMOTE_USER="crombo"
REMOTE_HOST="ftw3"

echo "==> Building crabjar-conductor release binary for commit $COMMIT..."
cargo build --release -p crabjar-conductor 2>&1 | tail -5

echo "==> SSH to ftw3, stopping conductor service (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "sudo dinitctl stop crabjar-conductor || true"

echo "==> rsync built artifact to ftw3: ${DEPLOY_DIR}/..."
rsync -avz --delete \
    target/release/crabjar-conductor \
    "${REMOTE_USER}@${REMOTE_HOST}:${DEPLOY_DIR}/crabjar-conductor"

echo "==> Deploying dinit service config with pesti-url..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} "cat > /tmp/conductor-service << 'EOF'
type     = process
command  = /opt/crabjar/conductor-start.sh
logfile  = /var/log/dinit/crabjar-conductor.log
depends-on: network.target
EOF
sudo cp /tmp/conductor-service /etc/dinit.d/crabjar-conductor"

echo "==> Deploying conductor startup script with pesti-url and jev-url..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} "cat > /tmp/conductor-start.sh << 'EOF'
#!/bin/bash
cd /opt/crabjar
export CRABJAR_JEV_API_URL=http://localhost:34159
exec ./crabjar-conductor --port 8091 --pesti-url http://127.0.0.1:8081
EOF"
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} "sudo mv /tmp/conductor-start.sh /opt/crabjar/conductor-start.sh && sudo chmod +x /opt/crabjar/conductor-start.sh"

echo "==> Reloading and starting conductor service on ftw3 (dinit)..."
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "sudo dinitctl reload crabjar-conductor && sudo dinitctl start crabjar-conductor"

echo "==> Verifying deployment..."
sleep 2
ssh -i "$SSH_KEY" ${REMOTE_USER}@${REMOTE_HOST} \
    "curl -s http://localhost:8091/workers | head -c 200"

echo ""
echo "==> Deployment complete. Commit: $COMMIT"
echo "==> Conductor API available at http://ftw3:8091 (via SSH tunnel)"
