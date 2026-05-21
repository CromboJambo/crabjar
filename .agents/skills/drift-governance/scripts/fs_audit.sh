#!/usr/bin/env bash
# drift-governance/scripts/fs_audit.sh
# Filesystem vs project_map discrepancy analysis
# Run before reading project_map.md to discover actual structure

set -euo pipefail

PROJECT_ROOT="${1:-$(pwd)}"
MAP_FILE="${PROJECT_ROOT}/project_map.md"

if [ ! -f "$MAP_FILE" ]; then
    echo "error: project_map.md not found at $MAP_FILE"
    exit 1
fi

echo "=== Drift Governance: Filesystem Audit ==="
echo "Project root: $PROJECT_ROOT"
echo "Map file: $MAP_FILE"

# Extract documented paths from project_map.md
echo ""
echo "=== Documented Paths (from project_map.md) ==="
grep -E '^\s+[-│├└]' "$MAP_FILE" | sed 's/[-│├└]/ /g' | awk '{print $NF}' | sort | uniq

# Actual filesystem structure
echo ""
echo "=== Actual Filesystem Structure ==="
find "$PROJECT_ROOT" -type f -name "*.rs" | sed "s|${PROJECT_ROOT}/||" | sort | uniq | head -50
find "$PROJECT_ROOT" -type d | sed "s|${PROJECT_ROOT}/||" | sort | uniq | head -50

echo ""
echo "=== Scan complete. Compare documented vs actual. ==="
echo "Run structure-auditor for detailed drift report."