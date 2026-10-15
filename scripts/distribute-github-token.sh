#!/bin/bash
# distribute-github-token.sh
# Distributes a GitHub PAT to crabjar-agent environment on all lab machines.
# Usage: ./distribute-github-token.sh <token> [machine...]
# If no machines specified, uses default fleet list.

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <github_token> [machine...]"
    echo "Example: $0 ghp_xxx ftw3 jambo sovnode"
    exit 1
fi

TOKEN="$1"
shift

# Default fleet list if no machines specified
if [ $# -eq 0 ]; then
    MACHINES="ftw3 jambo sovnode artix-box"
else
    MACHINES="$@"
fi

echo "=== Distributing GitHub PAT to crabjar-agent environment ==="
echo "Machines: ${MACHINES}"
echo ""

for MACHINE in ${MACHINES}; do
    echo "--- Updating ${MACHINE} ---"
    
    # Update .profile
    ssh crombo@${MACHINE} "sudo sed -i 's/export GITHUB_TOKEN=.*/export GITHUB_TOKEN=\"${TOKEN}\"/' /home/crabjar-agent/.profile" 2>/dev/null || \
        echo "  [WARN] Could not update .profile on ${MACHINE}"
    
    # Update environment.json
    ssh crombo@${MACHINE} "sudo sed -i 's/\"github_token\": \".*\"/\"github_token\": \"${TOKEN}\"/' /home/crabjar-agent/config/environment.json" 2>/dev/null || \
        echo "  [WARN] Could not update environment.json on ${MACHINE}"
    
    echo "  [OK] Token updated on ${MACHINE}"
done

echo ""
echo "=== Distribution complete ==="