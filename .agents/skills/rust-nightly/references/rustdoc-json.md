# Rustdoc JSON and MCP Integration

## rustdoc JSON output format

Nightly rustc supports JSON documentation output via unstable flags:

```bash
cargo +nightly rustc -- -Zunstable-options --output-format=json
```

Output goes to `target/doc/deps/` as `.json` files.

## rust-docs-mcp server

The `rust-docs-mcp` server provides MCP tools for querying Rust crate
documentation, dependencies, and module structure.

### Installation

```bash
# Requires nightly toolchain
rustup toolchain install nightly
cargo install rust-docs-mcp
```

### MCP tools available

| Tool | Purpose |
|------|---------|
| `cache_crate` | Cache a crate from crates.io, GitHub, or local path |
| `remove_crate` | Remove cached crate versions |
| `list_cached_crates` | List all cached crates |
| `list_crate_versions` | List versions of a specific crate |
| `get_crates_metadata` | Batch metadata queries |
| `search_items` | Full documentation search |
| `search_items_preview` | Lightweight search (IDs/names/types only) |
| `list_crate_items` | Browse all items in a crate |
| `get_item_details` | Detailed item info (signatures, fields, methods) |
| `get_item_docs` | Extract documentation string |
| `get_item_source` | View source code |
| `get_dependencies` | Dependency tree analysis |
| `structure` | Generate module hierarchy tree |

### MCP configuration

Add to your MCP config (e.g., `~/.config/opencode/mcp.json`):

```json
{
  "servers": {
    "rust-docs": {
      "command": "rust-docs-mcp",
      "transport": "stdio"
    }
  }
}
```

## crates.io API

For direct crates.io queries without MCP:

```bash
# Get crate metadata
curl -s "https://crates.io/api/v1/crates/{crate_name}"

# Get latest version
curl -s "https://crates.io/api/v1/crates/{crate_name}/versions" | head -1

# Download crate source
curl -sL "https://crates.io/api/v1/crates/{crate_name}/{version}/download" -o {crate}.crate
```

## lib.rs integration

lib.rs provides a web interface and API for Rust crate discovery:

- Browse: https://lib.rs
- Crate page: https://lib.rs/crates/{crate_name}
- API: https://lib.rs/api/v1/crates/{crate_name}

## GitHub rust-lang organization

Key repositories:

| Repo | Purpose |
|------|---------|
| `rust-lang/rust` | Compiler source |
| `rust-lang/cargo` | Package manager |
| `rust-lang/crates.io` | Registry |
| `rust-lang/rustup` | Toolchain manager |
| `rust-lang/std` | Standard library |
| `rust-lang/librustdoc` | rustdoc |

Search within rust-lang repos:

```
repo:rust-lang/rust {query}
repo:rust-lang/cargo {query}
```
