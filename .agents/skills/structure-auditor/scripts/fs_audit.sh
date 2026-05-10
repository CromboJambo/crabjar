#!/usr/bin/env bash
# filesystem vs project_map audit
set -euo pipefail

PROJECT_PATH="${1:-$PWD}"
PROJECT_MAP="${PROJECT_MAP:-$HOME/.mirror-lab/crabjar/project_map.md}"

if [[ ! -d "$PROJECT_PATH" ]]; then
    echo '{"error": "directory not found", "path": "'"$PROJECT_PATH"'"}'
    exit 1
fi

if [[ ! -f "$PROJECT_MAP" ]]; then
    echo '{"error": "project_map.md not found", "path": "'"$PROJECT_MAP"'"}'
    exit 1
fi

# Discover actual filesystem structure
actual="$(find "$PROJECT_PATH" -maxdepth 2 -type d | sort)"

# Extract documented structure from project_map
documented="$(grep -E '^\s+[-•]' "$PROJECT_MAP" | sed 's/^\s+[-•]\s//' | sort)"

# Compare
echo '{"actual_count": '$(echo "$actual" | wc -l)', "documented_count": '$(echo "$documented" | wc -l)'}'

# Find discrepancies
moved="$(comm -12 <(echo "$actual") <(echo "$documented") || true)"
missing="$(comm -23 <(echo "$actual") <(echo "$documented") || true)"
extra="$(comm -13 <(echo "$actual") <(echo "$documented") || true)"

if [[ -n "$missing" ]]; then
    echo '{"missing_from_map": "'"$missing"'"}'
fi
if [[ -n "$extra" ]]; then
    echo '{"extra_in_fs": "'"$extra"'"}'
fi
