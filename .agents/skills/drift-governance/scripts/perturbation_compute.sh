#!/usr/bin/env bash
# drift-governance/scripts/perturbation_compute.sh
# Compute bounded perturbation set for a given action
# Usage: perturbation_compute.sh <command> <undo_paths> <checksum_targets> <checkpoint_targets> <flight_recorder_targets> <data_integrity_targets>

set -euo pipefail

COMMAND="${1:-unknown}"
UNDO_PATHS="${2:-none}"
CHECKSUM_TARGETS="${3:-none}"
CHECKPOINT_TARGETS="${4:-none}"
FLIGHT_RECORDER_TARGETS="${5:-none}"
DATA_INTEGRITY_TARGETS="${6:-none}"

echo "=== Drift Governance: Perturbation Set ==="
echo "Command: $COMMAND"

# Count mitigable paths
MITIGABLE=0
UNMITIGABLE=0

if [ "$UNDO_PATHS" != "none" ]; then
    UNDO_COUNT=$(echo "$UNDO_PATHS" | tr ',' '\n' | wc -l)
    MITIGABLE=$((MITIGABLE + UNDO_COUNT))
    echo "  Undo paths: $UNDO_COUNT (mitigable)"
fi

if [ "$CHECKSUM_TARGETS" != "none" ]; then
    CHECKSUM_COUNT=$(echo "$CHECKSUM_TARGETS" | tr ',' '\n' | wc -l)
    MITIGABLE=$((MITIGABLE + CHECKSUM_COUNT))
    echo "  Checksum targets: $CHECKSUM_COUNT (mitigable)"
fi

if [ "$CHECKPOINT_TARGETS" != "none" ]; then
    CHECKPOINT_COUNT=$(echo "$CHECKPOINT_TARGETS" | tr ',' '\n' | wc -l)
    MITIGABLE=$((MITIGABLE + CHECKPOINT_COUNT))
    echo "  Checkpoint targets: $CHECKSUM_COUNT (mitigable)"
fi

if [ "$FLIGHT_RECORDER_TARGETS" != "none" ]; then
    FR_COUNT=$(echo "$FLIGHT_RECORDER_TARGETS" | tr ',' '\n' | wc -l)
    MITIGABLE=$((MITIGABLE + FR_COUNT))
    echo "  Flight recorder targets: $FR_COUNT (mitigable)"
fi

if [ "$DATA_INTEGRITY_TARGETS" != "none" ]; then
    DI_COUNT=$(echo "$DATA_INTEGRITY_TARGETS" | tr ',' '\n' | wc -l)
    MITIGABLE=$((MITIGABLE + DI_COUNT))
    echo "  Data integrity targets: $DI_COUNT (mitigable)"
fi

if [ "$UNDO_PATHS" = "none" ] && [ "$CHECKSUM_TARGETS" = "none" ] && [ "$CHECKPOINT_TARGETS" = "none" ] && [ "$FLIGHT_RECORDER_TARGETS" = "none" ] && [ "$DATA_INTEGRITY_TARGETS" = "none" ]; then
    UNMITIGABLE=1
    echo "  NoUndoPath: 1 (unmitigable)"
fi

TOTAL=$((MITIGABLE + UNMITIGABLE))
if [ "$TOTAL" -eq 0 ]; then
    BOUND="0.0"
elif [ "$MITIGABLE" -eq 0 ]; then
    BOUND="0.0"
else
    BOUND=$(echo "scale=2; $MITIGABLE / $TOTAL" | bc)
fi

echo ""
echo "=== Results ==="
echo "  Mitigable count: $MITIGABLE"
echo "  Unmitigable count: $UNMITIGABLE"
echo "  Bound: $BOUND"
echo "  has_unmitigable: $([ $UNMITIGABLE -gt 0 ] && echo 'true' || echo 'false')"

echo ""
echo "=== Interpretation ==="
if [ "$BOUND" = "0.0" ]; then
    echo "  Fully unmitigable — requires explicit permission"
elif [ "$BOUND" = "1.0" ]; then
    echo "  Fully mitigable — proceed with low risk"
else
    echo "  Partially mitigable — review required"
fi

echo ""
echo "=== Perturbation Set computed. Use for gate_check_with_reversibility. ==="