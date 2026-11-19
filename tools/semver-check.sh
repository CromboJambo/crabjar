#!/usr/bin/env bash
# tools/semver-check.sh — Validate semver discipline for crabjar workspace
# Usage: ./tools/semver-check.sh
# Exit 0 = pass, Exit 1 = fail (with details)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAILURES=0

echo "=== crabjar semver check ==="

# 1. Validate workspace version is valid semver
WS_VERSION=$(grep -A5 '^\[workspace.package\]' "$ROOT/Cargo.toml" | grep '^version' | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ -z "$WS_VERSION" ]]; then
    echo "❌ FAIL: No workspace version found in Cargo.toml"
    FAILURES=$((FAILURES + 1))
elif ! [[ "$WS_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "❌ FAIL: Workspace version '$WS_VERSION' is not valid semver (expected MAJOR.MINOR.PATCH)"
    FAILURES=$((FAILURES + 1))
else
    echo "✓ Workspace version: $WS_VERSION"
fi

# 2. Check Tier 1 leaf crates have explicit versions (not workspace = true)
LEAF_CRATES=(
    "crates/guard"
    "crates/sandbox"
    "crates/memory"
    "crates/crabjar/crabjar-web-search"
    "crates/crabjar/crabjar-web-extract"
    "crates/telemetry"
    "crates/terminal"
    "crates/crabjar/crabjar-memory"
)

echo ""
echo "Checking Tier 1 leaf crates:"
for crate_path in "${LEAF_CRATES[@]}"; do
    cargo_toml="$ROOT/$crate_path/Cargo.toml"
    if [[ ! -f "$cargo_toml" ]]; then
        echo "⚠ SKIP: $crate_path (not found)"
        continue
    fi
    
    crate_name=$(grep '^name' "$cargo_toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    
    # Check if using workspace version
    if grep -q '^version\.workspace = true' "$cargo_toml"; then
        echo "❌ FAIL: $crate_name uses version.workspace = true (must be explicit for independent publishing)"
        FAILURES=$((FAILURES + 1))
    else
        crate_version=$(grep '^version' "$cargo_toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
        if [[ -z "$crate_version" ]]; then
            echo "❌ FAIL: $crate_name has no version field"
            FAILURES=$((FAILURES + 1))
        elif ! [[ "$crate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "❌ FAIL: $crate_name version '$crate_version' is not valid semver"
            FAILURES=$((FAILURES + 1))
        else
            echo "✓ $crate_name: $crate_version"
        fi
    fi
done

# 3. Check for duplicate version tags in git history
echo ""
echo "Checking git tags:"
DUPLICATE_TAGS=$(git -C "$ROOT" tag | sort | uniq -d)
if [[ -n "$DUPLICATE_TAGS" ]]; then
    echo "❌ FAIL: Duplicate git tags found: $DUPLICATE_TAGS"
    FAILURES=$((FAILURES + 1))
else
    echo "✓ No duplicate tags"
fi

# 4. Check CHANGELOG has an entry for current workspace version
if [[ -n "$WS_VERSION" ]] && [[ -f "$ROOT/CHANGELOG.md" ]]; then
    if ! grep -q "v$WS_VERSION" "$ROOT/CHANGELOG.md"; then
        echo "⚠ WARN: CHANGELOG.md has no entry for v$WS_VERSION (add one before releasing)"
        # Warning only, not a failure — changelog might be added at release time
    else
        echo "✓ CHANGELOG has entry for v$WS_VERSION"
    fi
fi

echo ""
if [[ $FAILURES -eq 0 ]]; then
    echo "✅ All semver checks passed"
    exit 0
else
    echo "❌ $FAILURES semver check(s) failed"
    exit 1
fi
