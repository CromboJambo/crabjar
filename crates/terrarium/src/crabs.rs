/// Crab entity — a visual representation of a task attempt in the terrarium.
#[derive(Debug, Clone)]
pub struct Crab {
    pub id: String,
    pub zone: String,
    pub zone_xy: (i32, i32),
    pub behavior: String,
    pub tint: String,
    pub shell_age: u32,
    pub station_glyph: String,
    pub note: Option<String>,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_cents: f64,
}
