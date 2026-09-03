# Crabjar Glider

Conway's Game of Life with glider launcher - a terminal-based simulation for crabjar.

## Features

- **Classic glider pattern** - The iconic period-4 spaceship from Conway's Game of Life
- **Gosper glider gun** - Produces gliders every 36 generations
- **Puffer train** - Leaves a trail of beehives as it moves
- **Lightweight spaceship** - Horizontal movement variant
- **Benchmark mode** - Performance testing with multiple gliders
- **Interactive mode** - Real-time visualization with live editing

## Usage

```bash
# Quick simulation (100 generations)
cargo run -p crabjar-glider -m sim

# Benchmark mode (default: 1000 generations, 24 gliders)
cargo run -p crabjar-glider -m bench -g gospergun

# Custom parameters
cargo run -p crabjar-glider -m sim -w 120 -H 50 -f 30
```

### Command-line options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--mode` | `-m` | `sim` | `sim` (animation) or `bench` (performance test) |
| `--glider-type` | `-g` | `single` | Glider pattern: `single`, `gospergun`, `puffer`, `racer` |
| `--width` | `-w` | `80` | Grid width in cells |
| `--height` | `-H` | `40` | Grid height in cells |
| `--fps` | `-f` | `15` | Frames per second |

## Glider Patterns

### Single Glider (classic)
```
  █
 ██
█ █
██
```

### Gosper Glider Gun
Complex pattern that generates a new glider every 36 generations. Used to demonstrate self-reproducing systems in Game of Life.

### Puffer Train
Leaves a trail of stationary beehives as it moves diagonally across the grid.

### Lightweight Spaceship (racer)
Horizontal movement variant - faster than gliders but with different collision properties.

## Interactive Mode

Press these keys during simulation:

- **q** - Quit
- **g** - Spawn new glider at cursor position
- **+** - Increase speed (max 60 FPS)
- **-** - Decrease speed (min 5 FPS)
- **r** - Randomize grid with 25% density

## Performance

Benchmark results on RTX 4070 Ti SUPER:

```
Generations: 1000
Time: 0.386s
Gen/s: 2592.4
Gliders spawned: 24
Final population: 24
```

## Architecture

Self-contained binary with no external dependencies beyond workspace crates:

- `Grid` - 2D boolean array with Conway's rules implementation
- `Glider` - Pattern definitions with relative cell coordinates
- `Simulation` - Timing control and rendering (stdout/TUI)
- CLI parsing via `clap` with derive macros

## Integration

Part of the crabjar workspace. Follows project conventions:

- `snake_case` for functions/variables
- PascalCase for types
- Workspace Cargo.toml integration
- Uses `ratatui` for TUI rendering (when available)
- `crossterm` for terminal event handling

## Future Enhancements

- [ ] Multi-glider collision detection
- [ ] Pattern recognition and classification
- [ ] Save/load grid state to file
- [ ] Export animation to GIF/MP4
- [ ] WebAssembly support for browser-based simulation
