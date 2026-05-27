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
| tokio | 1.51.1 | Async runtime |
| serde | 1.0 | Serialization |
| serde_json | 1.0 | JSON output |
| toml | 0.8 | TOML config parsing |
| thiserror | 2.0 | Error handling |
| rusqlite | 0.37 | SQLite-backed knowledge store |
| clap | 4.5 | CLI parsing |
| tempfile | 3.24.0 | Integration test fixtures |
| reqwest | 0.13.2 | Networking |
| axum | 0.8 | HTTP server |
| candle-core | 0.10.2 | ML inference backend |
| burn | 0.21.0 | ML framework |
| safetensors | 0.7.0 | Model weight storage |
| cudarc | 0.19.4 | CUDA bindings |
| tiktoken-rs | 0.11.0 | Tokenization |
| tokenizers | 0.22.0 | HuggingFace tokenizers |
| rmcp | 1.7.0 | MCP protocol |
| ratatui | 0.30.0 | TUI framework |
| crossterm | 0.28 | Terminal events |
| uuid | 1.23.0 | UUID generation |
| clap_mangen | 0.2 | Man page generator |

## MSRV
not declared

## Release Profile
not declared

## Notes
- Edition: 2024
- Workspace members: crabjar-config, memory, orchestrator, guard, telemetry, sandbox, safetensors, tool_registry, codeburn-provider, codeburn-config, codeburn-classifier, codeburn-pricing, codeburn, skill-script-runner, skill-reference-store, llm-plug-in, llm-runner, zed-acp-bridge, zed-acp-server
- Runtime tool execution is intentionally disabled in this build
- All CLI responses are structured JSON on stdout
