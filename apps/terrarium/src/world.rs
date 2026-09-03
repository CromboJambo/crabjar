//! The terrarium world model — pure grid data, no rendering.
//!
//! This is the inside of the glass: cells, crabs, and their movement are
//! plain data. The presentation layer (ratty/RGP) reads this and emits
//! sequences; it never mutates it. Swapping the renderer later means writing
//! a new reader over this same state — the world logic does not change.
//!
//! Movement is *gliding*: each crab holds a fractional position that advances
//! toward its target cell at a fixed speed, so a renderer can place it
//! sub-cell (the RGP layer does this with `px`/`py` offsets). The world is
//! renderer-agnostic — it only knows the fractional position, not pixels.

/// Speed at which a crab glides toward its target, in cells per second.
const GLIDE_SPEED: f32 = 2.5;

/// Distance (in cells) below which a crab counts as arrived at its target.
const ARRIVE_EPS: f32 = 1e-3;

/// A crab wandering the grid.
#[derive(Debug, Clone)]
pub struct Crab {
    /// Stable RGP object id (also the crab's identity).
    pub id: u32,
    /// Fractional current column (sub-cell, for gliding).
    pub pos_col: f32,
    /// Fractional current row (sub-cell, for gliding).
    pub pos_row: f32,
    /// Target cell column the crab is heading toward.
    pub target_col: u16,
    /// Target cell row.
    pub target_row: u16,
    /// RGB color for placement.
    pub color: [u8; 3],
    /// Animation phase (radians-ish), advanced each tick.
    pub phase: f32,
    /// Seconds until the crab takes its next step.
    move_in: f32,
}

/// The grid world.
#[derive(Debug)]
pub struct World {
    /// Grid width in cells.
    pub width: u16,
    /// Grid height in cells.
    pub height: u16,
    /// Crabs living in the grid.
    pub crabs: Vec<Crab>,
    rng: u64,
    /// Monotonic tick counter (for the HUD).
    pub ticks: u64,
    /// Paused state — skip updates when true.
    pub paused: bool,
}

impl World {
    /// Create a world of `width`x`height` cells with `crab_count` crabs.
    #[must_use]
    pub fn new(width: u16, height: u16, crab_count: usize) -> Self {
        let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
        let palette: [[u8; 3]; 8] = [
            [255, 120, 60],
            [90, 200, 255],
            [120, 255, 140],
            [255, 220, 90],
            [255, 90, 180],
            [180, 130, 255],
            [230, 230, 230],
            [120, 220, 200],
        ];
        let mut crabs = Vec::with_capacity(crab_count);
        for i in 0..crab_count {
            let col = next_in_range(&mut rng, 0, width.saturating_sub(1));
            let row = next_in_range(&mut rng, 0, height.saturating_sub(1));
            crabs.push(Crab {
                id: 1000 + i as u32,
                pos_col: col as f32,
                pos_row: row as f32,
                target_col: col,
                target_row: row,
                color: palette[i % palette.len()],
                phase: i as f32 * 0.7,
                move_in: next_f32(&mut rng, 0.1, 0.8),
            });
        }
        Self {
            width,
            height,
            crabs,
            rng,
            ticks: 0,
            paused: false,
        }
    }

    /// Advance the world by `delta` seconds.
    pub fn tick(&mut self, delta: f32) {
        if self.paused { return; }
        self.ticks += 1;
        let width = self.width;
        let height = self.height;
        let mut rng = self.rng;
        for crab in &mut self.crabs {
            crab.phase += delta * 3.0;
            if crab.move_in > 0.0 {
                crab.move_in -= delta;
                continue;
            }
            let arrived = crab.at_target();
            if arrived {
                // Rest a beat, then pick a new neighbor to glide toward.
                pick_neighbor_target(crab, &mut rng, width, height);
                crab.move_in = next_f32(&mut rng, 0.25, 0.7);
            } else {
                glide_toward(crab, delta);
                // Keep moving until the glide completes.
                crab.move_in = 0.0;
            }
        }
        self.rng = rng;
    }

    /// Fast tick mode (10x speed) for + key
    pub fn tick_fast(&mut self) {
        for _ in 0..10 {
            self.tick(0.033); // Same as normal delta * 10
        }
    }

    /// Slow tick mode (0.5x speed) for - key
    pub fn tick_slow(&mut self) {
        self.tick(0.016); // Half the normal delta
    }

    /// Number of crabs.
    #[must_use]
    pub fn crab_count(&self) -> usize {
        self.crabs.len()
    }

    /// Toggle paused state.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Step forward one tick (for manual control).
    pub fn step(&mut self) {
        self.tick(0.033); // Single normal delta
    }
}

impl Crab {
    /// Whether the crab is (effectively) at its target cell.
    #[must_use]
    pub fn at_target(&self) -> bool {
        (self.pos_col - self.target_col as f32).abs() < ARRIVE_EPS
            && (self.pos_row - self.target_row as f32).abs() < ARRIVE_EPS
    }
}

/// Advance the crab's fractional position toward its target at GLIDE_SPEED.
/// The final step snaps exactly onto the target so the glide never overshoots.
fn glide_toward(crab: &mut Crab, delta: f32) {
    let step = GLIDE_SPEED * delta;
    let tx = crab.target_col as f32;
    let ty = crab.target_row as f32;

    let dc = tx - crab.pos_col;
    let dr = ty - crab.pos_row;
    if dc.abs() <= step {
        crab.pos_col = tx;
    } else {
        crab.pos_col += dc.signum() * step;
    }
    if dr.abs() <= step {
        crab.pos_row = ty;
    } else {
        crab.pos_row += dr.signum() * step;
    }
}

/// Pick a random in-bounds 4-neighbor as the crab's new target.
fn pick_neighbor_target(crab: &mut Crab, rng: &mut u64, width: u16, height: u16) {
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for _ in 0..8 {
        let (sx, sy) = DIRS[next_in_range(rng, 0, 3) as usize];
        let nx = crab.target_col as i32 + sx;
        let ny = crab.target_row as i32 + sy;
        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
            crab.target_col = nx as u16;
            crab.target_row = ny as u16;
            return;
        }
    }
}

// Deterministic LCG helpers — no `rand` dependency for a first cut.

fn lcg(rng: &mut u64) -> u64 {
    *rng = rng
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *rng >> 32
}

fn next_in_range(rng: &mut u64, lo: u16, hi: u16) -> u16 {
    let span = hi.saturating_sub(lo).max(1) as u64;
    lo + (lcg(rng) % span) as u16
}

fn next_f32(rng: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + ((lcg(rng) % 1000) as f32 / 1000.0) * (hi - lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_starts_with_requested_crab_count() {
        let w = World::new(20, 12, 6);
        assert_eq!(w.crab_count(), 6);
        assert_eq!(w.width, 20);
        assert_eq!(w.height, 12);
    }

    #[test]
    fn crabs_stay_in_bounds_over_many_ticks() {
        let mut w = World::new(10, 8, 5);
        for _ in 0..5000 {
            w.tick(0.1);
        }
        for c in &w.crabs {
            assert!(c.pos_col >= 0.0 && c.pos_col < w.width as f32);
            assert!(c.pos_row >= 0.0 && c.pos_row < w.height as f32);
        }
    }

    #[test]
    fn crabs_eventually_move() {
        let mut w = World::new(12, 10, 4);
        let start: Vec<(f32, f32)> = w
            .crabs
            .iter()
            .map(|c| (c.pos_col, c.pos_row))
            .collect();
        for _ in 0..2000 {
            w.tick(0.1);
        }
        let moved = w
            .crabs
            .iter()
            .zip(start.iter())
            .filter(|(c, s)| (c.pos_col, c.pos_row) != **s)
            .count();
        assert!(moved > 0, "at least one crab should have moved");
    }

    #[test]
    fn glide_reaches_target_exactly() {
        let mut w = World::new(8, 8, 1);
        let c = &mut w.crabs[0];
        c.pos_col = 1.0;
        c.pos_row = 1.0;
        c.target_col = 4;
        c.target_row = 3;
        c.move_in = 0.0;
        // Run until it arrives (bounded to avoid a hang in a regression).
        for _ in 0..1000 {
            if c.at_target() {
                break;
            }
            w.tick(0.1);
        }
        assert!(c.at_target(), "crab should land on its target");
        assert_eq!(c.pos_col, 4.0);
        assert_eq!(c.pos_row, 3.0);
    }

    #[test]
    fn glide_is_subcell_between_ticks() {
        let mut w = World::new(8, 8, 1);
        let c = &mut w.crabs[0];
        c.pos_col = 1.0;
        c.pos_row = 1.0;
        c.target_col = 5;
        c.target_row = 1;
        c.move_in = 0.0;
        w.tick(0.1);
        // One tick at 2.5 cells/s moves 0.25 cells — strictly between start and target.
        assert!(c.pos_col > 1.0 && c.pos_col < 5.0, "should be mid-glide");
        assert!((c.pos_col - 1.25).abs() < 1e-3, "expected ~0.25 cell step");
    }
}
