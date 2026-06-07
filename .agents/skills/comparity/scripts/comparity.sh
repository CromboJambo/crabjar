#!/usr/bin/env bash
# comparity — fetch metadata from a target repo for feature parity comparison
# Usage: comparity <url-or-path> [--depth <n>]
set -euo pipefail

TARGET="${1:?Usage: comparity <url-or-path> [--depth <n>]}"
DEPTH=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --depth) DEPTH="$2"; shift 2 ;;
    *) shift ;;
  esac
done

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

if [[ "$TARGET" == http* ]]; then
  echo "Cloning $TARGET..."
  CLONEDIR=$(mktemp -d)
  git clone --depth "$DEPTH" --filter=blob:none --sparse "$TARGET" "$CLONEDIR" 2>&1 | tail -3
  TARGET="$CLONEDIR"
fi

echo "=== README ==="
cat "$TARGET/README.md" 2>/dev/null | head -100 || echo "(no README)"

echo ""
echo "=== CARGO.TOML ==="
cat "$TARGET/Cargo.toml" 2>/dev/null | head -80 || \
cat "$TARGET/package.json" 2>/dev/null | head -80 || echo "(no manifest)"

echo ""
echo "=== AGENTS.md ==="
cat "$TARGET/AGENTS.md" 2>/dev/null | head -60 || \
cat "$TARGET/CLAUDE.md" 2>/dev/null | head -60 || echo "(no agent rules)"

echo ""
echo "=== PROJECT MAP ==="
cat "$TARGET/project_map.md" 2>/dev/null | head -80 || echo "(no project_map)"

echo ""
echo "=== DIRECTORY STRUCTURE ==="
find "$TARGET" -maxdepth 2 -type d ! -path '*/\.*' ! -path '*/target/*' ! -path '*/node_modules/*' ! -path '*/.git/*' | sort | head -40

if [[ "$TARGET" == *"$TMPDIR"* ]]; then
  echo ""
  echo "Cloned repo at: $TARGET (will be cleaned up)"
fi
