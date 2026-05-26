# Rust Nightly Toolchain

## Overview

Installs the Rust nightly toolchain via `rustup` and configures project-level
nightly pinning via `rust-toolchain.toml`.

## Prerequisites

Check if `rustup` is installed first:

```bash
rustup --version
```

If `rustup` is not installed, install it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then reload the shell:

```bash
source "$HOME/.cargo/env"
```

## Install nightly

```bash
rustup default nightly
```

## Install components

Nightly requires explicit component installation. Common components:

```bash
rustup component add rustfmt
rustup component add clippy
rustup component add rust-src          # for rustdoc JSON, macro expansion
rustup component add rustc-dev        # for compiling sysroot crates
rustup component add llvm-tools-preview # for coverage
rustup component add miri              # for undefined behavior detection
```

## Project-level nightly pin

Create or update `rust-toolchain.toml` in the project root:

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy", "rust-src"]
```

This pins the project to the specific nightly version (updated automatically
by rustup).

## Pin to a specific nightly date

For reproducible builds, pin to a date:

```bash
rustup toolchain install nightly-2025-06-24
```

Then in `rust-toolchain.toml`:

```toml
[toolchain]
channel = "nightly-2025-06-24"
components = ["rustfmt", "clippy", "rust-src"]
```

## Manage toolchains

```bash
rustup toolchain list           # list installed toolchains
rustup default nightly          # set default
rustup override set nightly     # project-level override
rustup override unset           # remove project override
rustup component list --installed  # list installed components
```

## Verify installation

```bash
rustc --version
cargo --version
rustup show
```

## After updating nightly

Nightly updates frequently. After a toolchain update:

1. `cargo clean` — clear stale artifacts
2. `cargo check` — verify compilation
3. `cargo clippy` — re-run lints (new nightly may introduce new lints)

## rustdoc JSON

For rustdoc JSON output (used by MCP servers and tooling):

```bash
cargo +nightly rustc -- -Zunstable-options --document-non-private-items --document-hidden-items --output-format=json
```

See `references/rustdoc-json.md` for MCP server integration details.

## Uninstall

```bash
rustup toolchain remove nightly
```
