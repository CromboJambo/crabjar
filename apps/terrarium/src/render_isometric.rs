//! Isometric renderer for CrabJar terrarium habitat.
//!
//! This module transforms the terrarium from ASCII art to isometric tile-based rendering.
//! Uses ANSI escape codes to draw diamond-shaped tiles that create a 3D isometric view.

use std::io::{self, Write};

// ============================================================================
// Constants
// ============================================================================

const TILE_WIDTH: i32 = 8;   // Width of diamond in characters
const TILE_HEIGHT: i32 = 4;  // Height of diamond in characters
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 24;

// ============================================================================
// Data Structures
// ============================================================================

/// Tile type (ground, water, grass, etc.)
#[derive(Debug, Clone, Copy)]
pub enum TileType {
    Grass,
    Water,
    Sand,
    Stone,
    Wood,
}

impl TileType {
    fn color(&self) -> &'static str {
        match self {
            TileType::Grass => "\x1b[32m",  // Green
            TileType::Water => "\x1b[36m",  // Cyan
            TileType::Sand => "\x1b[33m",   // Yellow
            TileType::Stone => "\x1b[90m",  // Gray
            TileType::Wood => "\x1b[95m",   // Purple
        }
    }

    fn fg_color(&self) -> &'static str {
        match self {
            TileType::Grass => "\x1b[42m",
            TileType::Water => "\x1b[46m",
            TileType::Sand => "\x1b[43m",
            TileType::Stone => "\x1b[40m",
            TileType::Wood => "\x1b[45m",
        }
    }
}

/// An isometric tile in the world
#[derive(Debug, Clone)]
pub struct IsometricTile {
    pub x: i32,      // Grid X coordinate
    pub y: i32,      // Grid Y coordinate
    pub z: i32,      // Height (for 3D effect)
    pub tile_type: TileType,
}

/// Entity that moves on top of tiles (crab, snake, etc.)
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub x: f32,      // Grid X coordinate (sub-cell, for smooth movement)
    pub y: f32,      // Grid Y coordinate
    pub z: f32,
    /// Wander velocity in grid cells per second (advanced by the sim loop).
    pub vx: f32,
    pub vy: f32,
    pub symbol: &'static str,
    pub color: &'static str,
}

/// Complete game world state
#[derive(Debug, Clone)]
pub struct GameWorld {
    /// Grid width in cells (entities live in 0..=width).
    pub width: i32,
    /// Grid height in cells (entities live in 0..=height).
    pub height: i32,
    pub tiles: Vec<IsometricTile>,
    pub entities: Vec<Entity>,
    pub tick: u64,
    pub paused: bool,
    pub speed: f32,
}

impl GameWorld {
    /// Find an entity by id.
    #[must_use]
    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }
}

// ============================================================================
// Isometric Projection Math
// ============================================================================

/// Convert isometric grid coordinates to screen (terminal) coordinates.
/// 
/// Standard isometric projection formula:
/// - screen_x = (grid_x - grid_y) * tile_width / 2
/// - screen_y = (grid_x + grid_y) * tile_height / 4 - grid_z * tile_height / 2
fn to_screen_coords(x: i32, y: i32, z: i32) -> (i32, i32) {
    let screen_x = (x - y) * TILE_WIDTH / 2;
    let screen_y = (x + y) * TILE_HEIGHT / 4 - z * TILE_HEIGHT / 2;
    (screen_x, screen_y)
}

/// Convert entity float coordinates to screen coordinates
fn to_screen_coords_f(x: f32, y: f32, z: f32) -> (i32, i32) {
    let ix = x as i32;
    let iy = y as i32;
    let iz = z as i32;
    to_screen_coords(ix, iy, iz)
}

// ============================================================================
// Rendering Functions
// ============================================================================

/// Draw a single diamond-shaped tile using ANSI escape codes.
fn draw_tile(tile: &IsometricTile, screen_x: i32, screen_y: i32) {
    let color = tile.tile_type.color();
    let fg_color = tile.tile_type.fg_color();
    let reset = "\x1b[0m";

    // Move cursor to tile position
    print!("\x1b[{};{}H", screen_y + 2, screen_x + 2);

    // Draw diamond using box-drawing characters
    // Top point
    print!("{} {}{}", color, "▲", reset);
    
    // Middle row (widest)
    print!("\x1b[{};{}H{}═══{}", screen_y + 3, screen_x + 1, color, reset);
    
    // Bottom point
    print!("\x1b[{};{}H{} {}{}", screen_y + 4, screen_x + 2, color, "▼", reset);

    // Add height indicator (for 3D effect)
    if tile.z > 0 {
        for i in 0..tile.z {
            print!("\x1b[{};{}H│{}", fg_color, screen_y + 5 + i, reset);
        }
    }
}

/// Draw an entity (crab, snake, etc.) at its position
fn draw_entity(entity: &Entity, screen_x: i32, screen_y: i32) {
    let reset = "\x1b[0m";
    
    print!("\x1b[{};{}H{}{}", screen_y + 2, screen_x + 2, entity.color, entity.symbol);
    print!("{}", reset);
}

/// Clear the entire screen and move cursor to home position
fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}

/// Draw the entire world state
pub fn render_world(world: &GameWorld) {
    // Clear screen
    clear_screen();

    // Render all tiles (sorted by z-height for proper depth ordering)
    let mut sorted_tiles = world.tiles.clone();
    sorted_tiles.sort_by_key(|t| t.z);

    for tile in &sorted_tiles {
        let (screen_x, screen_y) = to_screen_coords(tile.x, tile.y, tile.z);
        draw_tile(tile, screen_x, screen_y);
    }

    // Render all entities on top of tiles
    for entity in &world.entities {
        let (screen_x, screen_y) = to_screen_coords_f(entity.x, entity.y, entity.z);
        draw_entity(entity, screen_x, screen_y);
    }

    // Draw HUD
    draw_hud(world);

    io::stdout().flush().unwrap();
}

/// Draw heads-up display with game info
fn draw_hud(world: &GameWorld) {
    let reset = "\x1b[0m";
    let pos = SCREEN_WIDTH - 20;
    
    print!("\x1b[{};{}H=== CRABJAR HABITAT ==={}", pos, 1, reset);
    print!("\x1b[{};{}HTick: {} | Speed: {:.1}x", pos, 3, world.tick, world.speed);
    print!("\x1b[{};{}HEntities: {}", pos, 4, world.entities.len());
    print!("\x1b[{};{}HControls: q=quit, Space=pause, +=speed, -=slow{}", pos, 6, reset);
}

// ============================================================================
// World Generation
// ============================================================================

/// Generate a simple isometric grid for the habitat
pub fn generate_world(width: i32, height: i32) -> GameWorld {
    let mut tiles = Vec::new();
    
    // Create a grid of grass tiles with some variety
    for x in 0..width {
        for y in 0..height {
            let tile_type = match (x + y) % 5 {
                0 => TileType::Water,
                1 => TileType::Grass,
                2 => TileType::Sand,
                3 => TileType::Stone,
                _ => TileType::Wood,
            };
            
            tiles.push(IsometricTile {
                x,
                y,
                z: 0,
                tile_type,
            });
        }
    }

    // Add some elevated terrain (pyramids/structures)
    let mut elevated_tiles = vec![
        IsometricTile { x: 5, y: 5, z: 3, tile_type: TileType::Stone },
        IsometricTile { x: 10, y: 8, z: 2, tile_type: TileType::Wood },
        IsometricTile { x: 15, y: 5, z: 4, tile_type: TileType::Stone },
    ];
    
    tiles.append(&mut elevated_tiles);

    // Add entities (crabs) — each gets a small wander velocity.
    let entities = vec![
        Entity {
            id: "crab_001".to_string(),
            x: 3.0,
            y: 3.0,
            z: 0.0,
            vx: 0.5,
            vy: 0.3,
            symbol: "🦀",
            color: "\x1b[91m",  // Red
        },
        Entity {
            id: "crab_002".to_string(),
            x: 7.0,
            y: 5.0,
            z: 0.0,
            vx: -0.4,
            vy: 0.2,
            symbol: "🦀",
            color: "\x1b[94m",  // Blue
        },
        Entity {
            id: "crab_003".to_string(),
            x: 12.0,
            y: 7.0,
            z: 0.0,
            vx: 0.3,
            vy: -0.5,
            symbol: "🦀",
            color: "\x1b[92m",  // Green
        },
    ];

    GameWorld {
        width,
        height,
        tiles,
        entities,
        tick: 0,
        paused: false,
        speed: 1.0,
    }
}

// ============================================================================
// Simulation Step (shared by the render loop and any future sim driver)
// ============================================================================

/// Advance the world by `delta` seconds of game time. Returns true if an entity
/// moved (so callers can emit a DAGR event). Pure with respect to I/O — no
/// printing, no event construction; the caller decides what to do with motion.
pub fn step_world(world: &mut GameWorld, delta: f32) -> bool {
    if world.paused {
        return false;
    }
    world.tick += 1;

    let mut any_moved = false;
    for entity in &mut world.entities {
        // Integrate wander velocity (scaled by speed).
        entity.x += entity.vx * delta * world.speed;
        entity.y += entity.vy * delta * world.speed;

        // Bounce off the grid edges.
        if entity.x < 0.0 {
            entity.x = 0.0;
            entity.vx = entity.vx.abs();
        } else if entity.x > world.width as f32 {
            entity.x = world.width as f32;
            entity.vx = -entity.vx.abs();
        }
        if entity.y < 0.0 {
            entity.y = 0.0;
            entity.vy = entity.vy.abs();
        } else if entity.y > world.height as f32 {
            entity.y = world.height as f32;
            entity.vy = -entity.vy.abs();
        }

        any_moved = true;
    }
    any_moved
}

// ============================================================================
// Game Loop Integration
// ============================================================================

/// Main render loop for isometric terrarium
pub async fn run_isometric_world(mut world: GameWorld) {
    eprintln!("DEBUG: isometric render loop STARTED");
    
    let mut tick = 0u64;
    while world.tick > 0 || tick == 0 {
        if !world.paused {
            // Update world state
            world.tick += 1;
            
            // Move entities smoothly (simple interpolation) with DAGR emission
            use crate::dagr_feed::{EventSource, EventType, GameEvent};
            use uuid::Uuid;
            for entity in &mut world.entities {
                let old_x = entity.x;
                let old_y = entity.y;
                // Simple wandering behavior
                entity.x += 0.05_f32 * world.speed;
                entity.y += 0.03_f32 * world.speed;
                
                // Wrap around screen edges
                if entity.x > SCREEN_WIDTH as f32 {
                    entity.x = 0.0;
                }
                if entity.y > (SCREEN_HEIGHT * 2) as f32 {
                    entity.y = 0.0;
                }
                
                // Emit DAGR event if entity moved
                if (entity.x - old_x).abs() > 0.001 || (entity.y - old_y).abs() > 0.001 {
                    let event = GameEvent {
                        id: Uuid::new_v4().to_string(),
                        event_type: EventType::EntityMove {
                            entity_id: entity.id.clone(),
                            from_x: old_x,
                            from_y: old_y,
                            to_x: entity.x,
                            to_y: entity.y,
                            direction: None,
                        },
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        tick: world.tick,
                        source: EventSource::Simulation,
                        payload: crate::dagr_feed::EventPayload::EntityMove {
                            entity_id: entity.id.clone(),
                            from_x: old_x,
                            from_y: old_y,
                            to_x: entity.x,
                            to_y: entity.y,
                            direction: None,
                        },
                    };
                    eprintln!("DAGR EVENT: {}", serde_json::to_string(&event).unwrap_or_default());
                }
            }
            
            // Render frame
            render_world(&world);
        }

        let sleep_ms = (50.0 / world.speed).max(10.0) as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await; // speed-adjusted
    }
    
    eprintln!("DEBUG: isometric render loop EXITED");
}

// ============================================================================
// Standalone Demo Mode
// ============================================================================

/// Run a standalone demo without JSON-RPC control
pub fn run_demo() {
    let mut world = generate_world(20, 15);
    
    // Start animation immediately
    world.tick = 1;
    world.paused = false;
    
    eprintln!("🦀 CrabJar Isometric Habitat Demo");
    eprintln!("Press Ctrl+C to exit");
    
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_isometric_world(world));
}
