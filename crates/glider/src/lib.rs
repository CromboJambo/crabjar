//! Conway's Game of Life with glider support
//! 
//! A terminal-based simulation runner with real-time visualization.
//! Supports pre-built gliders, arbitrary patterns, and live editing.

mod grid;
mod glider;
mod simulation;
mod ui;
mod command;
mod error;

// Re-exports for convenience
pub use grid::Grid;
pub use glider::Glider;
pub use simulation::Simulation;
pub use error::{GliderError, GliderResult};
