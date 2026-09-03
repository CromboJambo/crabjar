//! Error types for crabjar-glider

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GliderError {
    #[error("Invalid glider pattern: {0}")]
    InvalidPattern(String),

    #[error("Grid out of bounds: ({x}, {y})")]
    OutOfBounds { x: usize, y: usize },

    #[error("Simulation error: {0}")]
    Simulation(String),
}

pub type GliderResult<T> = Result<T, GliderError>;
