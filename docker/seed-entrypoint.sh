#!/bin/bash
# Crabjar Adaptive Seed Container Entrypoint
# Detects environment, grows capabilities, reports to jambo, then runs crabjar.

set -e

echo "=== Crabjar Seed Container Starting ==="
echo "Timestamp: $(date -u)"
echo ""

# Step 1: Detect hardware environment
echo "--- Step 1: Hardware Detection ---"
CPU_CORES=$(nproc)
MEMORY_MB=$(grep MemTotal /proc/meminfo | awk '{print int($2/1024)}')
DISK_GB=$(df -BG / | grep -v Filesystem | awk '{print $2}' | tr -d 'G')

# GPU detection (if available)
GPU_INFO="none"
if command -v lspci &>/dev/null; then
    GPU_INFO=$(lspci | grep -i "3d\|nvidia\|amd\|radeon" || echo "none")
fi

echo "CPU cores: $CPU_CORES"
echo "Memory (MB): $MEMORY_MB"
echo "Disk (GB): $DISK_GB"
echo "GPU: $GPU_INFO"
echo ""

# Step 2: Report environment to jambo for review
echo "--- Step 2: Reporting to Jambo ---"
if [ -n "$JAMBO_REPORT_URL" ] && command -v curl &>/dev/null; then
    REPORT=$(cat <<EOF
{
    "container_id": "$(hostname)",
    "timestamp": "$(date -u +%s)",
    "cpu_cores": $CPU_CORES,
    "memory_mb": $MEMORY_MB,
    "disk_gb": $DISK_GB,
    "gpu_info": "$GPU_INFO",
    "status": "initialization_complete"
}
EOF
)
    
    echo "Sending report..."
    curl -s -X POST \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $JAMBO_TOKEN" \
        -d "$REPORT" \
        "$JAMBO_REPORT_URL" || echo "Warning: Failed to report to jambo"
    echo ""
fi

# Step 3: Grow capabilities based on environment
echo "--- Step 3: Growing Capabilities ---"

if [ "$CPU_CORES" -ge 8 ]; then
    echo "High CPU detected — installing parallel processing tools"
    apt-get update -qq && apt-get install -y --no-install-recommends -qq \
        parallel make gcc || true
fi

if [ "$MEMORY_MB" -gt 16384 ]; then
    echo "High memory detected — enabling larger model support"
    # Could install heavier dependencies here
fi

if echo "$GPU_INFO" | grep -qi "nvidia"; then
    echo "NVIDIA GPU detected — installing CUDA tools"
    apt-get update -qq && apt-get install -y --no-install-recommends -qq \
        nvidia-cuda-toolkit || true
fi

echo ""

# Step 4: Run setup wizard (interactive or config-driven)
echo "--- Step 4: Setup Wizard ---"
if [ ! -f "$CRABJAR_CONFIG_DIR/initialized" ]; then
    echo "First run — running setup wizard"
    
    # Could be interactive or read from env vars
    cat > "$CRABJAR_CONFIG_DIR/config.json" <<EOF
{
    "initialized": true,
    "cpu_cores": $CPU_CORES,
    "memory_mb": $MEMORY_MB,
    "gpu_info": "$GPU_INFO",
    "jambo_url": "$JAMBO_REPORT_URL"
}
EOF
    
    touch "$CRABJAR_CONFIG_DIR/initialized"
    echo "Setup complete — config written to $CRABJAR_CONFIG_DIR/config.json"
else
    echo "Already initialized — skipping setup wizard"
fi

echo ""
echo "=== Seed container ready ==="
echo ""

# Step 5: Run the actual crabjar command
exec "$@"