//! Glider pattern definitions

use serde::{Deserialize, Serialize};

pub type GliderResult<T> = Result<T, crate::GliderError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Glider {
    pub name: String,
    pub cells: Vec<(usize, usize)>, // relative coordinates from origin
}

impl Glider {
    /// Create a glider from type name
    pub fn from_type(name: &str) -> GliderResult<Self> {
        match name {
            "single" | "glider" => Ok(Self::classic()),
            "glidergun" | "gun" | "pulsar" => Ok(Self::gosper_gun()),
            "puffer" => Ok(Self::puffer()),
            "racer" => Ok(Self::lightweight_spaceship()),
            _ => Err(crate::GliderError::InvalidPattern(name.to_string())),
        }
    }

    /// Classic glider pattern (period 4, moves diagonally)
    pub fn classic() -> Self {
        Glider {
            name: "glider".to_string(),
            cells: vec![
                (1, 0),
                (2, 1),
                (0, 2),
                (1, 2),
                (2, 2),
            ],
        }
    }

    /// Gosper glider gun - produces gliders every 36 generations
    pub fn gosper_gun() -> Self {
        Glider {
            name: "gosper_gun".to_string(),
            cells: vec![
                (24, 0),
                (22, 1), (24, 1),
                (12, 2), (13, 2), (20, 2), (21, 2), (34, 2), (35, 2),
                (11, 3), (15, 3), (20, 3), (21, 3), (34, 3), (35, 3),
                (0, 4), (1, 4), (10, 4), (16, 4), (20, 4), (21, 4),
                (0, 5), (1, 5), (10, 5), (14, 5), (16, 5), (17, 5),
                (22, 5), (24, 5),
                (10, 6), (16, 6), (24, 6),
                (11, 7), (15, 7),
                (12, 8), (13, 8),
            ],
        }
    }

    /// Puffer train - leaves trail of beehives
    pub fn puffer() -> Self {
        Glider {
            name: "puffer".to_string(),
            cells: vec![
                (1, 0), (4, 0),
                (0, 1), (5, 1),
                (0, 2), (5, 2),
                (1, 3), (4, 3),
                (2, 4), (3, 4),
            ],
        }
    }

    /// Lightweight spaceship - moves horizontally
    pub fn lightweight_spaceship() -> Self {
        Glider {
            name: "racer".to_string(),
            cells: vec![
                (0, 1), (4, 1),
                (1, 2), (5, 2),
                (1, 3), (2, 3), (3, 3), (4, 3),
            ],
        }
    }

    /// Get glider's bounding box dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        let max_x = self.cells.iter().map(|(x, _)| x).max().copied().unwrap_or(0);
        let max_y = self.cells.iter().map(|(_, y)| y).max().copied().unwrap_or(0);
        (max_x + 1, max_y + 1)
    }
}
