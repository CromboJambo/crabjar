#!/usr/bin/env bash
# create new skill directory with SKILL.md skeleton
set -euo pipefail

SKILL_NAME="${1:-}"
SKILLS_DIR="${SKILLS_DIR:-$HOME/.agents/skills}"

if [[ -z "$SKILL_NAME" ]]; then
    echo '{"error": "skill name required", "usage": "skill_create.sh <name"}'
    exit 1
fi

if [[ "$SKILL_NAME" =~ [A-Z] ]]; then
    echo '{"error": "skill name must be lowercase with hyphens", "name": "'"$SKILL_NAME"'"}'
    exit 1
fi

if [[ ${#SKILL_NAME} -gt 64 ]]; then
    echo '{"error": "skill name must be under 64 characters", "name": "'"$SKILL_NAME"'"}'
    exit 1
fi

SKILL_DIR="$SKILLS_DIR/$SKILL_NAME"
mkdir -p "$SKILL_DIR/scripts" "$SKILL_DIR/references" "$SKILL_DIR/assets"

cat <<EOF > "$SKILL_DIR/SKILL.md"
---
name: ${SKILL_NAME}
description: |
  ...
---

# ${SKILL_NAME}

Instructions for the model...
EOF

echo '{"status": "created", "path": "'"$SKILL_DIR"'"}'
