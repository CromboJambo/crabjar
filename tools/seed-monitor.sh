#!/usr/bin/env bash
# crabjar-seed-monitor: TUI monitor for seed container reports on jambo
# Polls /api/reports endpoint and displays environment info in a live-updating table.

set -e

CONDUCTOR_URL="${1:-http://jambo:8091}"
REFRESH_INTERVAL="${2:-5}"

echo "Monitoring seed container reports from $CONDUCTOR_URL"
echo "Refresh interval: ${REFRESH_INTERVAL}s (Ctrl+C to exit)"
echo ""

while true; do
    clear
    
    # Fetch reports from conductor
    RESPONSE=$(curl -s "$CONDUCTOR_URL/api/reports" 2>/dev/null || echo '{"count":0,"reports":[]}')
    
    COUNT=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('count',0))" 2>/dev/null || echo "0")
    
    echo "═══════════════════════════════════════════════════════════════"
    echo " CRABJAR SEED CONTAINER MONITOR"
    echo "═══════════════════════════════════════════════════════════════"
    echo " Conductor: $CONDUCTOR_URL"
    echo " Reports received: $COUNT"
    echo " Last updated: $(date)"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    if [ "$COUNT" -eq 0 ]; then
        echo "No reports yet. Waiting for seed containers to report..."
        echo ""
        echo "Seed containers should POST to: $CONDUCTOR_URL/api/report"
    else
        # Display each report as a table row
        echo "$RESPONSE" | python3 -c '
import sys, json

data = json.load(sys.stdin)
reports = data.get("reports", [])

# Print header
print("{:<15} {:>4} {:>7} {:>8} {:>5} {}".format("HOSTNAME", "CPU", "RAM(GB)", "DISK(GB)", "GPU?", "CAPABILITIES"))
print("-" * 80)

for r in reports:
    hostname = r.get("hostname", "unknown")[:15]
    cpu = r.get("cpu_count", 0)
    ram_gb = r.get("memory_bytes", 0) // (1024**3)
    disk_gb = r.get("disk_bytes", 0) // (1024**3)
    has_gpu = "yes" if r.get("has_gpu", False) else "no"
    gpu_model = r.get("gpu_model", "")
    caps = ", ".join(r.get("growth_capabilities", []))
    
    print("{:<15} {:>4} {:>7} {:>8} {:>5} {}".format(hostname, cpu, ram_gb, disk_gb, has_gpu, caps))
    if gpu_model:
        print("              GPU: " + gpu_model)
' 2>/dev/null || echo "Failed to parse reports"
    fi
    
    echo ""
    echo "Refresh in ${REFRESH_INTERVAL}s... (Ctrl+C to exit)"
    
    sleep "$REFRESH_INTERVAL"
done
