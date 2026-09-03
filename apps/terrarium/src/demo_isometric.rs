//! Standalone demo binary for isometric terrarium rendering.
//!
//! This binary renders a static isometric habitat with moving crabs.
//! No JSON-RPC control, just pure visualization.

mod dagr_feed;
mod render_isometric;

use render_isometric::{generate_world, run_isometric_world};

#[tokio::main]
async fn main() {
    // Generate initial world state
    let mut world = generate_world(20, 15);
    
    // Start animation
    world.tick = 1;
    world.paused = false;
    
    println!("🦀 CrabJar Isometric Habitat Demo");
    println!("Press Ctrl+C to exit\n");
    
    // Run the render loop
    run_isometric_world(world).await;
}
