# Crabjar Demo Setup

## Quick Start (Manual)

The terrarium and glider are ready to run. Here's how to start them:

### 1. Create a herdr workspace

```bash
cd ~/projects/crabjar
herdr workspace create --cwd . --label "crabjar-demo" --no-focus
```

**Note:** Look for the `pane_id` in the output (e.g., `"pane_id":"w17:p1"`). Use that exact pane ID.

### 2. Split panes

```bash
# Replace w17 with your actual workspace number
herdr pane split w17:p1 --direction down
```

### 3. Launch terrarium in top pane

```bash
# Option A: Direct launch (blocks, best for viewing)
herdr pane send-text w17:p1 "./target/debug/crabjar-terrarium"

# Option B: Background with nohup
herdr pane send-text w17:p1 "nohup ./target/debug/crabjar-terrarium > /tmp/terrarium.log 2>&1 &"
```

### 4. Launch glider in bottom pane

```bash
# Simple simulation (100 generations)
herdr pane send-text w17:p2 "./target/debug/crabjar-glider -m sim"

# Or benchmark mode (1000 generations, gosper gun)
herdr pane send-text w17:p2 "./target/debug/crabjar-glider -m bench -g gospergun"
```

## Viewing Output

```bash
# Check terrarium output
herdr pane read w17:p1 --source visible --lines 50

# Check glider output
herdr pane read w17:p2 --source visible --lines 50
```

## Expected Outputs

### Terrarium (40x20 grid with crabs)
```
........................................
..............🦀.........................
...............................🦀........
... etc ...
🦀 Terrarium v0.1 | Crabs: 6 | Ticks: 42
Controls: q=quit, Space=pause, +=speed, -=slow
Status: Running in text mode (no RGP)
```

### Glider (benchmark output)
```
Generation 100: population = 5
Generation 200: population = 5
...
Benchmark complete!
Generations: 1000
Time: 0.386s
Gen/s: 2592.4
Gliders spawned: 24
Final population: 24
```

## Troubleshooting

### "No such device" error
- The terrarium queries terminal size which fails in herdr panes
- **Fix:** Use text mode explicitly: `TERR_MODE=text ./target/debug/crabjar-terrarium`

### Pane ID confusion
- Each `herdr workspace create` generates a new pane ID (w12, w13, w14, etc.)
- **Always use the pane ID from your latest workspace creation output**

### Process doesn't start
- Herdr panes may block on stdin/stdout
- **Try:** `nohup ./target/debug/crabjar-terrarium > /tmp/out.log 2>&1 &`

## Alternative: Run in Your Current Terminal

If herdr panes are tricky, just run directly:

```bash
# Terrarium
./target/debug/crabjar-terrarium

# Glider (simulation)
./target/debug/crabjar-glider -m sim

# Glider (benchmark)
./target/debug/crabjar-glider -m bench -g gospergun
```

## Files Created

- `demo-herdr.sh` - Script to automate workspace setup (see above for usage)
- `HERDR_INTEGRATION.md` - Detailed documentation of herdr integration
