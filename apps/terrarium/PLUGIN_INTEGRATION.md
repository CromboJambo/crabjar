# Terrarium + Glider as Herdr Plugins

## Overview

Both `crabjar-terrarium` and `crabjar-glider` now have **Herdr plugin modes** that run as stdio JSON-RPC servers. This is the "official" plugin route that avoids the hard blocks with multiplexing.

## Reference: herdr-flock

[`herdr-flock`](https://github.com/ragamo/herdr-flock) is a working reference implementation:
- **Rust-based** using `ratatui 0.29 + crossterm 0.28` (same stack as ours)
- **SQLite persistence** via `rusqlite`
- **Auto-discovers herdr socket**, falls back to demo mode
- **Live agent state visualization** with sheep avatar for each session

Our approach follows the same pattern.

## Plugin Manifest Format

Both plugins use a `herdr-plugin.toml` manifest:

```toml
id = "terrarium.habitat"
name = "Terrarium Habitat"
version = "0.1.0"
min_herdr_version = "0.7.0"
description = "Text-mode terrarium simulation with crabs and gliders"
platforms = ["linux", "macos"]

[[build]]
command = ["cargo", "build", "--release", "-p", "crabjar-app-terrarium"]
platforms = ["linux", "macos"]

[[panes]]
id = "habitat"
title = "Terrarium Habitat"
placement = "split"
command = ["./target/release/crabjar-terrarium-plugin"]

[[actions]]
id = "open_habitat"
title = "Open Terrarium Habitat"
contexts = ["workspace"]
```

## Installation

### Option A: Link as Herdr Plugin (Recommended)

```bash
# From terrarium directory
cd ~/projects/crabjar/apps/terrarium
herdr plugin link .

# From glider directory  
cd ~/projects/crabjar/crates/glider
herdr plugin link .
```

This registers the plugins in herdr's config and adds them to your plugin list.

### Option B: Manual Invocation (Quick Test)

```bash
# Start terrarium plugin
./target/release/crabjar-terrarium-plugin stdio

# Send JSON-RPC command via stdin
echo '{"id":1,"method":"terrarium/start","params":{"action":"start"}}' | ./target/release/crabjar-terrarium-plugin stdio
```

## JSON-RPC Protocol

Both plugins accept commands over stdin/stdout:

### Request Format

```json
{
  "id": 1,
  "method": "terrarium/start",
  "params": {
    "action": "Start"
  }
}
```

### Terrarium Commands

| Method | Params | Description |
|--------|--------|-------------|
| `terrarium/start` | `{}` | Start simulation (unpause) |
| `terrarium/stop` | `{}` | Stop simulation |
| `terrarium/pause` | `{}` | Pause animation |
| `terrarium/resume` | `{}` | Resume animation |
| `terrarium/set_speed` | `{"value": "10.0"}` | Set speed multiplier |
| `terrarium/step` | `{}` | Advance one tick |

### Glider Commands

| Method | Params | Description |
|--------|--------|-------------|
| `glider/start` | `{}` | Start simulation |
| `glider/stop` | `{}` | Stop simulation |
| `glider/pause` | `{}` | Pause animation |
| `glider/resume` | `{}` | Resume animation |
| `glider/set_mode` | `{"value": "sim"}` | Set simulation mode |
| `glider/step` | `{}` | Advance one generation |

### Response Format

```json
{
  "id": 1,
  "result": {
    "status": "started",
    "message": "Terrarium started",
    "crabs_count": 42
  },
  "error": null
}
```

## Architecture

### Plugin Binary (`plugin.rs`)

Each plugin has two concurrent tasks:

1. **Command Handler** (stdio JSON-RPC server)
   - Reads commands from stdin
   - Parses JSON-RPC requests
   - Updates shared state
   - Writes responses to stdout

2. **Render Loop** (separate async task)
   - Polls shared state for changes
   - Renders ASCII art/emoji to stdout
   - Runs at fixed FPS (20 for terrarium, 10 for glider)

### State Sharing

Both tasks share a mutable `State` struct:

```rust
struct TerrariumState {
    running: bool,
    paused: bool,
    speed_multiplier: f64,
    crabs_count: usize,
}
```

The command handler updates fields; the render loop reads them.

## Next Steps

### Phase 1: Working Prototype ✅ DONE
- [x] Build stdio JSON-RPC plugin binaries
- [x] Create `herdr-plugin.toml` manifests
- [x] Test basic start/stop/pause/resume commands

### Phase 2: Full Integration
- [ ] Wire up real terrarium world logic (from `apps/terrarium/src/world.rs`)
- [ ] Wire up real glider simulation (from `crates/glider/src/glider.rs`)
- [ ] Add keyboard controls via stdin events (if herdr supports them)
- [ ] Persist state between sessions

### Phase 3: Advanced Features
- [ ] Multi-pane setup (terrarium + glider side-by-side)
- [ ] Shared world state (gliders interact with crabs)
- [ ] Bevy window for GPU-accelerated 3D view (optional)
- [ ] Stats overlay pane

## Comparison: herdr-flock vs Our Approach

| Feature | herdr-flock | Our Terrarium/Glider |
|---------|-------------|---------------------|
| Language | Rust | Rust |
| UI Library | ratatui 0.29 | ratatui 0.30 |
| Rendering | Pixel-art sheep | Emoji + ASCII art |
| Persistence | SQLite (flock.db) | None yet (volatile) |
| Controls | Mouse + keyboard | JSON-RPC only (initially) |
| Modes | Demo fallback | Text mode fallback |

## Verification

Test that the plugins run:

```bash
# Check binaries exist
ls -la target/release/crabjar-terrarium-plugin target/release/crabjar-glider-plugin

# Test help output
./target/release/crabjar-terrarium-plugin --help
./target/release/crabjar-glider-plugin --help

# Quick test (manual JSON-RPC)
echo '{"id":1,"method":"terrarium/start","params":{"action":"start"}}' | ./target/release/crabjar-terrarium-plugin stdio &
sleep 2
kill %1
```

## Files Modified/Created

| File | Purpose |
|------|---------|
| `apps/terrarium/src/plugin.rs` | Plugin binary (stdio JSON-RPC server) |
| `apps/terrarium/Cargo.toml` | Added plugin deps (`tokio`, `serde_json`) |
| `apps/terrarium/herdr-plugin.toml` | Herdr plugin manifest |
| `crates/glider/src/plugin.rs` | Plugin binary (stdio JSON-RPC server) |
| `crates/glider/Cargo.toml` | Added plugin deps (`tokio`) |
| `crates/glider/herdr-plugin.toml` | Herdr plugin manifest |

## References

- [`herdr-flock`](https://github.com/ragamo/herdr-flock) — Reference implementation
- [`herdr-agent-state`](~/.hermes/plugins/herdr-agent-state/) — Installed herdr plugin in our setup
- [Herdr Plugin Docs](https://herdr.dev/docs/plugins/) — Official documentation (if available)
