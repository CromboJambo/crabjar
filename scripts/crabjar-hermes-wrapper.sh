#!/bin/bash
# crabjar-hermes-wrapper.sh
# Runs hermes-agent within the crabjar environment with proper telemetry.
# This enables full transparency and reproducibility for agent operations.

set -e

# Crabjar environment variables
export AGENT_USER="crabjar-agent"
export AGENT_HOME="/home/crabjar-agent"
export CRABJAR_INBOX="${AGENT_HOME}/inbox"
export CRABJAR_OUTBOX="${AGENT_HOME}/outbox"
export CRABJAR_LOG="${AGENT_HOME}/log"
export CRABJAR_WORKSPACE="${AGENT_HOME}/workspace"
export CRABJAR_CONFIG="${AGENT_HOME}/config"

# Telemetry configuration
export HERMES_TELEMETRY_ENABLED="true"
export HERMES_TELEMETRY_OUTPUT="${CRABJAR_LOG}/hermes-telemetry.jsonl"

# Ensure log directory exists (use sudo if needed)
if [ ! -d "${CRABJAR_LOG}" ]; then
    mkdir -p "${CRABJAR_LOG}" 2>/dev/null || sudo mkdir -p "${CRABJAR_LOG}"
fi

echo "=== Crabjar Hermes Environment ==="
echo "Agent user: ${AGENT_USER}"
echo "Home: ${AGENT_HOME}"
echo "Inbox: ${CRABJAR_INBOX}"
echo "Outbox: ${CRABJAR_OUTBOX}"
echo "Log: ${CRABJAR_LOG}"
echo "Workspace: ${CRABJAR_WORKSPACE}"
echo "Telemetry: ${HERMES_TELEMETRY_OUTPUT}"
echo ""

# Run hermes with the crabjar environment
exec hermes "$@"