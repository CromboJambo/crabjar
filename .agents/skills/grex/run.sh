#!/bin/bash

# Grex Skill: Pattern Verification Engine - Execution Layer
# This script implements the verification logic defined in SKILL.md

set -e

# Parse arguments
TARGET_PATH="$1"
PATTERN="$2"
MODE="$3"
CONTEXT_LIMIT="${4:-0}"  # Default to 0 (all lines)

# Validate inputs
if [[ -z "$TARGET_PATH" || -z "$PATTERN" || -z "$MODE" ]]; then
    echo '{"status": "FAILURE", "match_count": 0, "line_numbers": [], "error_message": "Missing required arguments: target_path, pattern, or mode"}'
    exit 1
fi

# Check if file exists
if [[ ! -f "$TARGET_PATH" ]]; then
    echo '{"status": "FAILURE", "match_count": 0, "line_numbers": [], "error_message": "Target file not found: '"$TARGET_PATH"'"'
    exit 1
fi

# Execute grep based on mode
if [[ "$MODE" == "EXISTS" ]]; then
    # Look for pattern
    MATCHES=$(grep -n "$PATTERN" "$TARGET_PATH" 2>/dev/null || true)

    if [[ -z "$MATCHES" ]]; then
        echo '{"status": "FAILURE", "match_count": 0, "line_numbers": [], "error_message": "Pattern not found in target_path."}'
    else
        # Extract line numbers
        LINE_NUMBERS=()
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                LINE_NUMBERS+=("${line%%:*}")
            fi
        done <<< "$MATCHES"

        echo '{"status": "SUCCESS", "match_count": '${#LINE_NUMBERS[@]}', "line_numbers": '"$(printf '%s,' "${LINE_NUMBERS[@]}" | sed 's/,$//')"'}'
    fi
elif [[ "$MODE" == "ABSENT" ]]; then
    # Ensure pattern is NOT present
    MATCHES=$(grep -n "$PATTERN" "$TARGET_PATH" 2>/dev/null || true)

    if [[ -n "$MATCHES" ]]; then
        echo '{"status": "FAILURE", "match_count": 0, "line_numbers": [], "error_message": "Pattern should not be present in target_path."}'
    else
        echo '{"status": "SUCCESS", "match_count": 0, "line_numbers": [], "error_message": null}'
    fi
else
    echo '{"status": "FAILURE", "match_count": 0, "line_numbers": [], "error_message": "Invalid mode. Use EXISTS or ABSENT."}'
    exit 1
fi
