#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HERMES_DIR="${HERMES_HOME:-$HOME/.hermes}"
SKILLS_DIR="$HERMES_DIR/skills"

echo "Installing crabjar skills to $SKILLS_DIR"
echo "==========================================="

# Create skills directory if needed
mkdir -p "$SKILLS_DIR"

# Copy each skill
for skill_file in "$SCRIPT_DIR"/*.md; do
    skill_name="$(basename "$skill_file" .md)"
    dest="$SKILLS_DIR/${skill_name}.md"
    cp "$skill_file" "$dest"
    echo "  [OK] $skill_name → $dest"
done

echo ""
echo "Done. Skills are now available in ~/.hermes/skills/"
echo "They will be loaded by Hermes on next conversation."
