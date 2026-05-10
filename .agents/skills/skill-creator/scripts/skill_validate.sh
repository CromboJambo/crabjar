#!/usr/bin/env bash
# validate skill directory structure
set -euo pipefail

SKILL_PATH="${1:-}"

if [[ -z "$SKILL_PATH" ]]; then
    echo '{"error": "skill path required", "usage": "skill_validate.sh <path"}'
    exit 1
fi

if [[ ! -d "$SKILL_PATH" ]]; then
    echo '{"error": "directory not found", "path": "'"$SKILL_PATH"'"}'
    exit 1
fi

errors=0

# Check SKILL.md exists
if [[ ! -f "$SKILL_PATH/SKILL.md" ]]; then
    echo "missing: SKILL.md"
    errors=$((errors + 1))
fi

# Check frontmatter name matches directory
if [[ -f "$SKILL_PATH/SKILL.md" ]]; then
    dir_name="$(basename "$SKILL_PATH")"
    skill_name="$(grep -m1 '^name:' "$SKILL_PATH/SKILL.md" | sed 's/^name: //' | tr -d ' \t')"
    if [[ "$dir_name" != "$skill_name" ]]; then
        echo "name mismatch: directory=$dir_name, frontmatter=$skill_name"
        errors=$((errors + 1))
    fi
fi

# Check description present
if [[ -f "$SKILL_PATH/SKILL.md" ]]; then
    if ! grep -q '^description:' "$SKILL_PATH/SKILL.md"; then
        echo "missing: description in frontmatter"
        errors=$((errors + 1))
    fi
fi

# Check name constraints
if [[ -f "$SKILL_PATH/SKILL.md" ]]; then
    skill_name="$(grep -m1 '^name:' "$SKILL_PATH/SKILL.md" | sed 's/^name: //' | tr -d ' \t')"
    if [[ "$skill_name" =~ [A-Z] ]]; then
        echo "name uppercase: must be lowercase with hyphens"
        errors=$((errors + 1))
    fi
    if [[ ${#skill_name} -gt 64 ]]; then
        echo "name too long: must be under 64 characters"
        errors=$((errors + 1))
    fi
fi

if [[ $errors -gt 0 ]]; then
    echo '{"status": "invalid", "errors": '$errors'}'
    exit 1
else
    echo '{"status": "valid"}'
fi
