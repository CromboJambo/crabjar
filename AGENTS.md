# Repository Guidelines

## Project Structure & Module Organization

CrabJar is a Rust 2024 workspace centered on the `crabjar` CLI:

```text
src/main.rs                  # CLI entry point and command dispatch
src/lib.rs                   # shared library surface for the binary
src/project_loader.rs        # config loading
src/state_docs.rs            # state-doc and overlay handling
src/knowledge_store/         # knowledge-store command bridge
src/crabjar-config/          # workspace config crate
memory/                      # agent-context crate, SQLite-backed storage
orchestrator/, guard/        # supporting workspace crates
tests/cli.rs                 # integration tests against the compiled binary
state-docs/                  # project state documentation and overlays
```

## Build, Test, and Development Commands

Use `just` for common workflows:

- `just check`: runs `cargo check --workspace`.
- `just build`: runs `cargo build -p crabjar` to compile the CLI.
- `just run state list`: runs the binary with replaceable arguments.
- `just test`: runs `cargo test --workspace`.
- `just clean`: removes Cargo build artifacts.
Use raw Cargo for narrower scope, for example `cargo test -p crabjar`.

## Coding Style & Naming Conventions

Run `cargo fmt` before submitting changes. Use `cargo clippy -- -D warnings` and fix warnings. Follow standard Rust naming: `snake_case` for functions, variables, and modules; `PascalCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants.

Use `thiserror` for library errors and propagate failures with `?`. Avoid `unwrap()` outside tests. CLI commands should emit structured JSON to stdout; do not add plain-text success paths.

## Testing Guidelines

Use `#[test]` and `#[tokio::test]`. Place unit tests beside the code under `#[cfg(test)]`; place CLI integration tests in `tests/cli.rs` using `std::process::Command`.

Filesystem fixtures must use `tempfile::tempdir()` and must not write into the repository. New CLI subcommands and state-doc operations need happy-path and error-path tests. Test names should describe behavior, for example `state_list_returns_json`.

## Commit & Pull Request Guidelines

Recent commits use imperative subjects such as `Add comprehensive guides for bounded agent workflows` and `Update REPRO.md with current project details`. Start with a capital verb, keep the subject under about 72 characters, and add a body when context matters.

Pull requests should explain what changed and why, link relevant context, and keep refactors separate from features. Before review, run `just check`, `just test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`.

## Architecture Notes

Workspace config comes from `.crabjar_config.toml`; missing or malformed config is a soft failure and should leave `workspace: null`. State docs live under `state-docs/`, with overlay JSON sidecars in `state-docs/overlay/`. Detection is not authorization: observer crates may report, but must not execute actions.

## Agent Autonomy Constraints
- Agent should never execute sudo commands — present them as user-run actions
- Detection ≠ authorization: observer reports must not trigger execution
- Reversibility gating: destructive actions require user permission
- Commands requiring root access are categorical user-run only

## Tool Override
- Content search: Use Bash with `rg` (ripgrep) instead of the Grep tool
- File search: Use Glob (NOT find or ls)
- Read files: Use Read (NOT cat/head/tail)
- Edit files: Use Edit (NOT sed/awk)
- Write files: Use Write (NOT echo >/cat <<EOF)
- Communication: Output text directly (NOT echo/printf)

## Wasm Dependency Stripping
- `zed-acp-bridge` Wasm compilation requires minimal dependency set: `zed_extension_api`, `serde`, `uuid(js)` only
- `tokio` pulls `mio` (wasm incompatible) — must disable net feature or exclude entirely
- `rusqlite` pulls `libsqlite3-sys` (C compilation fails on wasm) — incompatible with Wasm
- `uuid` requires `js` feature for wasm-compatible RNG (v4 disabled on wasm)
- HTTP (axum) cannot be adapted to stdio — requires dedicated stdio server for Zed agent protocol
