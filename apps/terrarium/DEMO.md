# Terrarium Demo Script for Herdr

## Quick Start

```bash
cd ~/projects/crabjar

# Create a new herdr workspace
herdr workspace create --cwd . --label terrarium-demo

# Send the demo command (replace w1C:p1 with your actual pane ID)
herdr pane send-text w1C:p1 'cd /home/crombo/projects/crabjar && script -q -c "./target/release/crabjar-terrarium-plugin stdio" /tmp/herdr_terram.txt <<EOF\n{"id":1,"method":"terrarium/start","params":{"action":"start"}}\nsleep 8\nexit'

# After a few seconds, read the output
sleep 10
herdr pane read w1C:p1 --source visible --lines 50
```

## How It Works

1. **PTY Allocation**: `script` command allocates a pseudo-terminal for the plugin
2. **JSON-RPC Control**: Commands are piped in via stdin
3. **Render Loop**: Plugin runs at ~20 FPS, outputting ASCII art frames
4. **Output Capture**: herdr captures PTY output which includes the rendered frames

## Expected Output

```
🦀 Terrarium plugin started (stdio mode)
DEBUG: handle_commands STARTED
DEBUG: received command: {"id":1,"method":"terrarium/start","params":{"action":"start"}}
{"id":1,"result":{"status":"started","message":"Terrarium started","crabs_count":0},"error":null}
DEBUG: render_loop STARTED
DEBUG: render_loop tick=1 speed=1
🦀 Terrarium - Tick: 1 | Speed: 1x─────────────────────────────────────🐍 Snake moving... (placeholder)─────────────────────────────────────Controls: q=quit, Space=pause, +=speed, -=slow
DEBUG: render_loop tick=2 speed=1
🦀 Terrarium - Tick: 2 | Speed: 1x─────────────────────────────────────🐍 Snake moving... (placeholder)─────────────────────────────────────Controls: q=quit, Space=pause, +=speed, -=slow
... (continues for 8 seconds = ~160 frames)
```

## Files Created/Modified

- `apps/terrarium/src/plugin.rs` - Plugin binary with JSON-RPC server
- `apps/terrarium/Cargo.toml` - Added plugin dependencies
- `apps/terrarium/herdr-plugin.toml` - Herdr plugin manifest
- `apps/terrarium/PLUGIN_INTEGRATION.md` - Full integration documentation

## Next Steps

1. **Wire up real world logic** - Integrate with `world.rs` for actual snake movement
2. **Add keyboard controls** - Send Space, +, - commands via JSON-RPC
3. **Multi-pane setup** - Run both terrarium and glider in separate panes
4. **Persist state** - Add SQLite database like herdr-flock

## Verification Commands

```bash
# Check binaries exist
ls -la target/release/crabjar-terrarium-plugin target/release/crabjar-glider-plugin

# Test plugin directly
echo '{"id":1,"method":"terrarium/start","params":{"action":"start"}}' | timeout 5 ./target/release/crabjar-terrarium-plugin stdio 2>&1 | head -10

# Check herdr status
herdr status

# List workspaces
herdr workspace list
```
