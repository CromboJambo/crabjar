#!/usr/bin/env bash
# Terrarium Phase 2 Demo — recordable via `script`

cd /home/crombo/projects/terrarium

echo "🦀 Terrarium Phase 2 — Isometric Habitat Simulator"
echo ""
echo "Running with demo contract..."
echo ""

# Run terrarium in non-interactive mode to show output
timeout 3 ./target/release/terrarium view demo-run.json --behaviors behaviors.toml || true

echo ""
echo "Demo complete!"
