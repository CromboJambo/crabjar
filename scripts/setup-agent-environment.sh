#!/bin/bash
# crabjar-agent-environment-setup.sh
# Standardizes the crabjar-agent user environment across lab machines.
# Run as root or with sudo on each target machine.

set -e

AGENT_USER="crabjar-agent"
AGENT_HOME="/home/${AGENT_USER}"

echo "=== Crabjar Agent Environment Setup ==="
echo "Target: ${AGENT_USER}@$(hostname)"
echo ""

# Check if agent user exists, create if not
if id "${AGENT_USER}" &>/dev/null; then
    echo "[OK] User ${AGENT_USER} already exists"
else
    echo "[CREATE] Creating system user ${AGENT_USER}..."
    # On Artix/dinit systems, use useradd; on systemd systems, adduser may be available
    if command -v useradd &>/dev/null; then
        useradd --system --shell /bin/bash --create-home "${AGENT_USER}"
    elif command -v adduser &>/dev/null; then
        adduser --system --shell /bin/bash --home "${AGENT_HOME}" --no-create-home "${AGENT_USER}"
        mkdir -p "${AGENT_HOME}"
        chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}"
    else
        echo "[ERROR] Neither useradd nor adduser found"
        exit 1
    fi
    echo "[OK] User ${AGENT_USER} created"
fi

# Create standard directory structure
echo ""
echo "Setting up home directory structure..."

mkdir -p "${AGENT_HOME}/workspace"
mkdir -p "${AGENT_HOME}/inbox"
mkdir -p "${AGENT_HOME}/outbox"
mkdir -p "${AGENT_HOME}/log"
mkdir -p "${AGENT_HOME}/config"
mkdir -p "${AGENT_HOME}/cache"
mkdir -p "${AGENT_HOME}/.ssh"

# Set ownership
chown -R "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}"

echo "[OK] Directory structure created"

# Create SSH directory with proper permissions
chmod 700 "${AGENT_HOME}/.ssh"
echo "[OK] SSH directory configured"

# Create .profile for environment setup
if [ ! -f "${AGENT_HOME}/.profile" ]; then
    cat > "${AGENT_HOME}/.profile" << 'EOF'
# Crabjar Agent Environment Profile
export HOME=/home/crabjar-agent
export AGENT_USER=crabjar-agent
export AGENT_HOME=/home/crabjar-agent

# Standard paths
export PATH="/usr/local/bin:/usr/bin:/bin:/opt/crabjar:${PATH}"

# Agent-specific environment variables
export CRABJAR_INBOX=${AGENT_HOME}/inbox
export CRABJAR_OUTBOX=${AGENT_HOME}/outbox
export CRABJAR_LOG=${AGENT_HOME}/log
export CRABJAR_WORKSPACE=${AGENT_HOME}/workspace
export CRABJAR_CONFIG=${AGENT_HOME}/config

# Shell settings
export PS1="[${AGENT_USER}@$(hostname) \W]$ "
EOF
    chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/.profile"
    echo "[OK] .profile created"
else
    echo "[OK] .profile already exists"
fi

# Create .bashrc for interactive sessions
if [ ! -f "${AGENT_HOME}/.bashrc" ]; then
    cat > "${AGENT_HOME}/.bashrc" << 'EOF'
# Crabjar Agent Bash Configuration
source ~/.profile

# History settings
export HISTSIZE=1000
export HISTFILESIZE=2000
export HISTFILE=${AGENT_HOME}/.bash_history

# Prompt for interactive sessions
if [ -t 1 ]; then
    export PS1="[${AGENT_USER}@$(hostname) \W]$ "
fi
EOF
    chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/.bashrc"
    echo "[OK] .bashrc created"
else
    echo "[OK] .bashrc already exists"
fi

# Create basic agent config file
if [ ! -f "${AGENT_HOME}/config/agent.json" ]; then
    cat > "${AGENT_HOME}/config/agent.json" << EOF
{
  "user": "${AGENT_USER}",
  "hostname": "$(hostname)",
  "version": "1.0.0",
  "environment": {
    "inbox": "${AGENT_HOME}/inbox",
    "outbox": "${AGENT_HOME}/outbox",
    "log": "${AGENT_HOME}/log",
    "workspace": "${AGENT_HOME}/workspace"
  },
  "created": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
    chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/config/agent.json"
    echo "[OK] agent.json created"
else
    echo "[OK] agent.json already exists"
fi

# Create basic environment config file
if [ ! -f "${AGENT_HOME}/config/environment.json" ]; then
    # Detect architecture
    ARCH=$(uname -m)
    
    # Detect init system
    if command -v dinitctl &>/dev/null; then
        INIT_SYSTEM="dinit"
    elif command -v systemctl &>/dev/null; then
        INIT_SYSTEM="systemd"
    else
        INIT_SYSTEM="unknown"
    fi
    
    cat > "${AGENT_HOME}/config/environment.json" << EOF
{
  "hostname": "$(hostname)",
  "architecture": "${ARCH}",
  "init_system": "${INIT_SYSTEM}",
  "os": "$(cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"' || echo 'unknown')",
  "created": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
    chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/config/environment.json"
    echo "[OK] environment.json created"
else
    echo "[OK] environment.json already exists"
fi

echo ""
echo "=== Setup Complete ==="
echo "Agent user: ${AGENT_USER}"
echo "Home directory: ${AGENT_HOME}"
echo ""
echo "Directory structure:"
ls -la "${AGENT_HOME}/" | grep -v "^total"
echo ""
echo "To test the environment, run:"
echo "  sudo -u ${AGENT_USER} bash -c 'env | grep AGENT'"