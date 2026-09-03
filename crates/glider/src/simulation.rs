//! Simulation engine with timing control

use std::time::Instant;

use crate::{Grid, GliderResult};

pub struct Simulation {
    pub grid: Grid,
    fps: u64,
    last_frame: Instant,
}

impl Simulation {
    pub fn new(grid: Grid, fps: u64) -> Self {
        Simulation {
            grid,
            fps,
            last_frame: Instant::now(),
        }
    }

    pub fn step(&mut self) {
        self.grid.step();
    }

    pub fn set_fps(&mut self, fps: u64) {
        self.fps = fps;
    }

    pub fn get_fps(&self) -> u64 {
        self.fps
    }

    pub fn get_cursor_pos(&self) -> (usize, usize) {
        // Default center position for now
        (self.grid.width / 2, self.grid.height / 2)
    }

    /// Render to stdout (non-interactive mode)
    pub fn render_stdout(&self) -> GliderResult<()> {
        let width = self.grid.width;
        let height = self.grid.height;

        // Clear screen
        print!("\x1b[2J\x1b[H");

        // Print header with population count
        println!("Generation: {}", 0); // TODO: track generation count
        println!("Population: {}", self.grid.population());
        println!();

        // Render grid
        for y in 0..height {
            for x in 0..width {
                if self.grid.at(x, y) {
                    print!("█");
                } else {
                    print!("·");
                }
            }
            println!();
        }

        Ok(())
    }

    /// Render to TUI (interactive mode)
    pub fn render_tui(&mut self) -> GliderResult<()> {
        let width = self.grid.width;
        let height = self.grid.height;

        // Clear screen
        print!("\x1b[2J\x1b[H");

        // Render with color and info panel
        println!("\x1b[36m╔════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[36m║  🐍 Crabjar Glider v0.1              ║\x1b[0m");
        println!("\x1b[36m╚════════════════════════════════════════╝\x1b[0m");
        println!();

        // Render grid with live cells in green
        for y in 0..height {
            print!("\x1b[32m");
            for x in 0..width {
                if self.grid.at(x, y) {
                    print!("▓");
                } else {
                    print!("·");
                }
            }
            print!("\x1b[0m");
            println!();
        }

        // Info panel
        println!();
        println!("\x1b[90mPopulation: {}\x1b[0m", self.grid.population());
        println!("\x1b[90mFPS: {}\x1b[0m", self.fps);
        println!("\x1b[90mControls: q=quit, g=spawn glider, +=speed-\x1b[0m");

        Ok(())
    }
}
