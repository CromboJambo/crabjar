//! Grid representation for Conway's Game of Life

use serde::{Deserialize, Serialize};

use crate::{Glider, GliderError, GliderResult};

#[derive(Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<bool>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![false; height]; width];
        Grid { width, height, cells }
    }

    pub fn at(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.cells[x][y]
    }

    pub fn set(&mut self, x: usize, y: usize, alive: bool) {
        if x < self.width && y < self.height {
            self.cells[x][y] = alive;
        }
    }

    pub fn spawn_glider(&mut self, glider: &Glider, cx: usize, cy: usize) {
        for (dx, dy) in glider.cells.iter().copied() {
            let x = cx.wrapping_add(dx);
            let y = cy.wrapping_add(dy);
            if x < self.width && y < self.height {
                self.cells[x][y] = true;
            }
        }
    }

    pub fn randomize(&mut self, density: usize) {
        let total = (self.width * self.height) as f64;
        for x in 0..self.width {
            for y in 0..self.height {
                self.cells[x][y] = rand::random::<f64>() < (density as f64 / total);
            }
        }
    }

    pub fn population(&self) -> usize {
        self.cells.iter().flatten().filter(|&&c| c).count()
    }

    /// Compute next generation
    pub fn step(&mut self) {
        let width = self.width;
        let height = self.height;
        let mut next = vec![vec![false; height]; width];

        for x in 0..width {
            for y in 0..height {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells[x][y];

                // Conway's rules:
                // 1. Live cell with 2-3 neighbors survives
                // 2. Dead cell with 3 neighbors becomes alive
                // 3. All others die/stay dead
                next[x][y] = if alive {
                    neighbors == 2 || neighbors == 3
                } else {
                    neighbors == 3
                };
            }
        }

        self.cells = next;
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x as i32 + dx) as usize;
                let ny = (y as i32 + dy) as usize;

                if nx < self.width && ny < self.height {
                    if self.cells[nx][ny] {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    /// Get bounding box of all live cells
    pub fn bounding_box(&self) -> Option<(usize, usize, usize, usize)> {
        let mut min_x = self.width;
        let mut max_x = 0;
        let mut min_y = self.height;
        let mut max_y = 0;
        let mut found = false;

        for x in 0..self.width {
            for y in 0..self.height {
                if self.cells[x][y] {
                    found = true;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }

        if found {
            Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
        } else {
            None
        }
    }
}
