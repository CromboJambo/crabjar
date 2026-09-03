//! Conway's Game of Life with glider launcher - self-contained binary

use clap::Parser;
use std::time::{Duration, Instant};

// Grid representation
#[derive(Clone)]
struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Vec<bool>>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![false; height]; width];
        Grid { width, height, cells }
    }

    fn at(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height { return false; }
        self.cells[x][y]
    }

    fn set(&mut self, x: usize, y: usize, alive: bool) {
        if x < self.width && y < self.height { self.cells[x][y] = alive; }
    }

    fn spawn_glider(&mut self, glider: &Glider, cx: usize, cy: usize) {
        for (dx, dy) in glider.cells.iter().copied() {
            let x = cx.wrapping_add(dx);
            let y = cy.wrapping_add(dy);
            if x < self.width && y < self.height { self.cells[x][y] = true; }
        }
    }

    fn randomize(&mut self, density: usize) {
        let total = (self.width * self.height) as f64;
        for x in 0..self.width {
            for y in 0..self.height {
                self.cells[x][y] = rand::random::<f64>() < (density as f64 / total);
            }
        }
    }

    fn population(&self) -> usize {
        self.cells.iter().flatten().filter(|&&c| c).count()
    }

    fn step(&mut self) {
        let width = self.width;
        let height = self.height;
        let mut next = vec![vec![false; height]; width];

        for x in 0..width {
            for y in 0..height {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells[x][y];
                next[x][y] = if alive { neighbors == 2 || neighbors == 3 } else { neighbors == 3 };
            }
        }
        self.cells = next;
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if dx == 0 && dy == 0 { continue; }
                let nx = (x as i32 + dx) as usize;
                let ny = (y as i32 + dy) as usize;
                if nx < self.width && ny < self.height && self.cells[nx][ny] { count += 1; }
            }
        }
        count
    }
}

// Glider pattern definitions
#[derive(Clone)]
struct Glider {
    name: String,
    cells: Vec<(usize, usize)>,
}

impl Glider {
    fn from_type(name: &str) -> Result<Glider, String> {
        match name {
            "single" | "glider" => Ok(Self::classic()),
            "gospergun" | "gun" | "pulsar" => Ok(Self::gosper_gun()),
            "puffer" => Ok(Self::puffer()),
            "racer" => Ok(Self::spaceship()),
            _ => Err(format!("Unknown glider type: {}", name)),
        }
    }

    fn classic() -> Self {
        Glider {
            name: "glider".to_string(),
            cells: vec![(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)],
        }
    }

    fn gosper_gun() -> Self {
        Glider {
            name: "gosper_gun".to_string(),
            cells: vec![
                (24, 0), (22, 1), (24, 1), (12, 2), (13, 2), (20, 2), (21, 2), (34, 2), (35, 2),
                (11, 3), (15, 3), (20, 3), (21, 3), (34, 3), (35, 3),
                (0, 4), (1, 4), (10, 4), (16, 4), (20, 4), (21, 4),
                (0, 5), (1, 5), (10, 5), (14, 5), (16, 5), (17, 5),
                (22, 5), (24, 5), (10, 6), (16, 6), (24, 6),
                (10, 7), (16, 7), (24, 7), (11, 8), (15, 8), (12, 9), (13, 9),
            ],
        }
    }

    fn puffer() -> Self {
        Glider {
            name: "puffer".to_string(),
            cells: vec![(1, 0), (4, 0), (0, 1), (5, 1), (0, 2), (5, 2), (1, 3), (4, 3), (2, 4), (3, 4)],
        }
    }

    fn spaceship() -> Self {
        Glider {
            name: "spaceship".to_string(),
            cells: vec![(0, 1), (4, 1), (1, 2), (5, 2), (1, 3), (2, 3), (3, 3), (4, 3)],
        }
    }
}

// Simulation engine
struct Simulation {
    grid: Grid,
    fps: u64,
}

impl Simulation {
    fn new(grid: Grid, fps: u64) -> Self {
        Simulation { grid, fps }
    }

    fn step(&mut self) { self.grid.step(); }

    fn set_fps(&mut self, fps: u64) { self.fps = fps; }

    fn get_fps(&self) -> u64 { self.fps }

    fn get_cursor_pos(&self) -> (usize, usize) {
        (self.grid.width / 2, self.grid.height / 2)
    }

    fn render_stdout(&self) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.grid.width;
        let height = self.grid.height;
        print!("\x1b[2J\x1b[H");
        println!("Generation: 0 | Population: {}", self.grid.population());
        for y in 0..height {
            for x in 0..width {
                if self.grid.at(x, y) { print!("█"); } else { print!("·"); }
            }
            println!();
        }
        Ok(())
    }

    fn render_tui(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.grid.width;
        let height = self.grid.height;
        print!("\x1b[2J\x1b[H");
        println!("\x1b[36m╔════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[36m║  🐍 Crabjar Glider v0.1              ║\x1b[0m");
        println!("\x1b[36m╚════════════════════════════════════════╝\x1b[0m");
        println!();
        for y in 0..height {
            print!("\x1b[32m");
            for x in 0..width {
                if self.grid.at(x, y) { print!("▓"); } else { print!("·"); }
            }
            print!("\x1b[0m");
            println!();
        }
        println!();
        println!("\x1b[90mPopulation: {}\x1b[0m", self.grid.population());
        println!("\x1b[90mFPS: {}\x1b[0m", self.fps);
        Ok(())
    }
}

// CLI and main
#[derive(clap::Parser)]
#[command(name = "crabjar-glider")]
struct Cli {
    #[arg(short = 'm', long, default_value = "sim")]
    mode: SimulationMode,
    
    #[arg(short = 'g', long, default_value = "single")]
    glider_type: String,
    
    #[arg(short = 'w', long, default_value = "80")]
    width: usize,
    
    #[arg(short = 'H', long, default_value = "40")]
    height: usize,
    
    #[arg(short = 'f', long, default_value = "15")]
    fps: u64,
}

#[derive(clap::Parser, Clone)]
enum SimulationMode {
    Sim { #[arg(long, default_value_t = false)] interactive: bool },
    Bench { #[arg(short, long, default_value = "1000")] generations: usize },
}

impl std::str::FromStr for SimulationMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sim" => Ok(SimulationMode::Sim { interactive: false }),
            "bench" => Ok(SimulationMode::Bench { generations: 1000 }),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.mode {
        SimulationMode::Sim { interactive } => {
            if interactive { run_interactive(&cli)?; } else { run_brief(&cli)?; }
        },
        SimulationMode::Bench { generations } => { run_benchmark(generations, &cli.glider_type)?; }
    }
    Ok(())
}

fn run_brief(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let mut grid = Grid::new(cli.width, cli.height);
    let (cx, cy) = (cli.width / 2, cli.height / 2);
    let glider = Glider::from_type(&cli.glider_type)?;
    grid.spawn_glider(&glider, cx, cy);

    println!("🐍 Glider simulation starting...");
    println!("Grid: {}x{}, FPS: {}", cli.width, cli.height, cli.fps);
    
    let mut sim = Simulation::new(grid, cli.fps);
    for g in 0..100 {
        sim.step();
        if g % 10 == 0 { sim.render_stdout()?; }
    }

    println!("\nSimulation complete!");
    println!("Generations run: 100");
    println!("Gliders spawned: 1");
    println!("Final population: {}", sim.grid.population());
    Ok(())
}

fn run_interactive(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let mut grid = Grid::new(cli.width, cli.height);
    let (cx, cy) = (cli.width / 2, cli.height / 2);
    let glider = Glider::from_type(&cli.glider_type)?;
    grid.spawn_glider(&glider, cx, cy);

    let mut sim = Simulation::new(grid, cli.fps);
    
    println!("Interactive mode - Press:");
    println!("  q: quit");
    println!("  g: spawn glider at cursor");
    println!("  +: increase speed");
    println!(" -: decrease speed");
    println!(" r: randomize grid");

    crossterm::terminal::enable_raw_mode()?;
    
    loop {
        sim.render_tui()?;
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => break,
                    crossterm::event::KeyCode::Char('g') => {
                        let (x, y) = sim.get_cursor_pos();
                        let glider = Glider::from_type(&cli.glider_type)?;
                        sim.grid.spawn_glider(&glider, x, y);
                    }
                    crossterm::event::KeyCode::Char('+') => { sim.set_fps(sim.get_fps().min(60)); }
                    crossterm::event::KeyCode::Char('-') => { sim.set_fps(sim.get_fps().max(5)); }
                    crossterm::event::KeyCode::Char('r') => { sim.grid.randomize(cli.width * cli.height / 4); }
                    _ => {}
                }
            }
        }
    }
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn run_benchmark(generations: usize, glider_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli {
        mode: SimulationMode::Bench { generations },
        glider_type: glider_type.to_string(),
        width: 80, height: 40, fps: 60,
    };

    let mut grid = Grid::new(cli.width, cli.height);
    let glider = Glider::from_type(glider_type)?;
    for x in [10, 30, 50, 70] {
        for y in [10, 20, 30] { grid.spawn_glider(&glider, x, y); }
    }

    let mut sim = Simulation::new(grid, cli.fps);
    let start = Instant::now();
    
    for g in 0..generations {
        sim.step();
        if g % 100 == 0 && g > 0 {
            println!("Generation {}: population = {}", g, sim.grid.population());
        }
    }
    
    let elapsed = start.elapsed();
    
    println!("\nBenchmark complete!");
    println!("Generations: {}", generations);
    println!("Time: {:.3}s", elapsed.as_secs_f32());
    println!("Gen/s: {:.1}", generations as f64 / elapsed.as_secs_f64());
    println!("Gliders spawned: 24");
    println!("Final population: {}", sim.grid.population());
    Ok(())
}
