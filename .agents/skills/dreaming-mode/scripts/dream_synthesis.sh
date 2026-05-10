#!/usr/bin/env bash
# reflection synthesis for dreaming mode
set -euo pipefail

STAGING_DIR="${STAGING_DIR:-$HOME/.mirror-lab/staging}"
OUTPUT_DIR="${OUTPUT_DIR:-$HOME/.mirror-lab/crabjar}"

if [[ ! -d "$STAGING_DIR" ]]; then
    echo '{"error": "staging directory not found", "path": "'"$STAGING_DIR"'"}'
    exit 1
fi

# Gather staged events
events="$(find "$STAGING_DIR" -type f | sort)"
count="$(echo "$events" | wc -l)"

if [[ $count -eq 0 ]]; then
    echo '{"status": "empty staging", "events": 0}'
    exit 0
fi

# Synthesize patterns
for event in $events; do
    echo "--- event: $event ---"
    head -10 "$event"
done

# Write synthesis to crabjar
synthesis_file="$OUTPUT_DIR/session_summary.md"
cat <<EOF > "$synthesis_file"
# Session Summary

> Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
> Source: $STAGING_DIR

## Patterns Identified
List of recurring patterns from staged events.

## Structural Shifts
List of architectural changes observed.

## Proposed Updates
List of proposed updates to agent_config.md or project_map.md.

---
*End of synthesis.*
EOF

echo '{"status": "synthesized", "path": "'"$synthesis_file"'"}'
