#!/usr/bin/env bash
# Install Rust nightly toolchain with common components
set -euo pipefail

echo "=== Rust Nightly Installer ==="

# Check for rustup
if ! command -v rustup &>/dev/null; then
    echo "rustup not found. Installing..."
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "Then: source \"\$HOME/.cargo/env\""
    exit 1
fi

# Install nightly
echo "Installing nightly toolchain..."
rustup default nightly

# Install common components
echo "Installing components..."
rustup component add rustfmt clippy rust-src 2>/dev/null || true

# Verify
echo ""
echo "Installed toolchains:"
rustup toolchain list
echo ""
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"
echo ""
echo "Done. To pin a project to nightly, create rust-toolchain.toml:"
echo '  [toolchain]'
echo '  channel = "nightly"'
echo '  components = ["rustfmt", "clippy", "rust-src"]'
