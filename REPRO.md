# Reproducing the crabjar /home/crombo/crabjar workflow

## Source
The directory `/home/crombo/crabjar` contains crabjar, a stripped-down Rust CLI for local state-docs management. Upstream: https://github.com/crabjar/crabjar.

## Reproduction Steps
1. Clone: `git clone https://github.com/crabjar/crabjar`
2. Build: `just build` or `cargo build -p crabjar`
3. Run: `just run state list` or `cargo run -p crabjar -- state list`
4. Test: `just test` or `cargo test --workspace`

## Key Dependencies (Cargo.toml)
| Crate | Version | Purpose |
| tokio | 1.35 | Async runtime |
| serde | 1.0 | Serialization |
| serde_json | 1.0 | JSON output |
| toml | 0.8 | TOML config parsing |
| thiserror | 2.0 | Error handling |
| rusqlite | 0.32 | SQLite-backed knowledge store |
| clap | 4.5 | CLI parsing |
| tempfile | 3.14 | Integration test fixtures |
| reqwest | 0.12 | Networking |
| axum | 0.7 | HTTP server |

## MSRV
not declared

## Release Profile
not declared

## Notes
- Edition: 2024
- Workspace members: crabjar-config, memory, orchestrator, codeburn-provider, skill-script-runner, skill-reference-store, guard
- Runtime tool execution is intentionally disabled in this build
- All CLI responses are structured JSON on stdout
