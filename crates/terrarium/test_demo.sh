#!/usr/bin/env bash
cd /home/crombo/projects/terrarium

echo "🦀 Terrarium Phase 2 — Isometric Habitat"
echo ""
echo "Demo contract: demo-run.json (4 tasks: 1 done, 1 failed, 1 blocked, 1 queued)"
echo ""

# Run with --help to show usage first
./target/release/terrarium --help || true

echo ""
echo "---"
echo ""
echo "Running headless view..."

# Since the TUI blocks, let's just verify it builds and works via tests
cargo test --lib 2>&1 | tail -5
