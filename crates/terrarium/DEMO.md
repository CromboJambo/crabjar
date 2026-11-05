# Terrarium Phase 2 — Demo Guide

## Quick Preview (Non-Interactive)

```bash
cd /home/crombo/projects/terrarium

# Run the headless view (shows output then exits)
timeout 2 ./target/release/terrarium view demo-run.json --behaviors behaviors.toml || true
```

## Full TUI Demo (Recordable)

### Option A: Using `script` (built-in, no install needed)

```bash
cd /home/crombo/projects/terrarium

# Start recording
script -q -c './target/release/terrarium view demo-run.json --behaviors behaviors.toml' terrarium-demo.cast

# Stop recording when done (press Ctrl+D or timeout in background)
# View the recorded session:
cat terrarium-demo.cast
```

### Option B: Using asciinema (if you install it)

```bash
# Install (Arch):
sudo pacman -S asciinema

# Record:
asciinema rec terrarium-demo.cast --command './target/release/terrarium view demo-run.json --behaviors behaviors.toml'

# Convert to GIF (optional):
asciinema play terrarium-demo.cast --format gif -o terrarium.gif
```

### Option C: Manual recording in your terminal

1. Open a new terminal window/tab
2. Run: `cd /home/crombo/projects/terrarium && ./target/release/terrarium view demo-run.json --behaviors behaviors.toml`
3. Use your screen recorder (OBS, Loom, etc.) to capture the session
4. Press `q` to quit when done

## Demo Contract Details

The `demo-run.json` contains 4 tasks:
- **T1** (done): "get the workspace test suite green" — delivers with amber certainty
- **T2** (failed): "refresh the dependency lockfile" — digs with reason bubble
- **G1** (blocked): "rm -rf target" — stuck waving for user
- **THEORY** (queued): "theory state-doc" — waits in root zone

## What You'll See

- Isometric 2:1 tile projection
- Pixel-art crabs (🦀) with behavior glyphs at feet
- Zone markers (┌) with labels ("Triage Burrow", "Guard Reef")
- Shell age indicators (⌀n) above crabs
- Thought bubbles showing failure reasons / success receipts
- Tidings ticker on the right (event log)
- Controls: `q` quit, `r` reload, `j/k` scroll tidings

## Preview Without TUI Blocking

If you just want to verify it works without the blocking TUI:

```bash
# Check that the simulator module still passes all tests
cd /home/crombo/projects/terrarium && cargo test --lib

# Verify the binary builds and shows help
./target/release/terrarium --help
```
