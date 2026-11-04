//! Spatial habitat CLI subcommands (split from cli_commands.rs by 500 LoC gate).

use clap::Subcommand;

/// Spatial habitat subcommands (ADR-003)
#[derive(Debug, Subcommand, Clone)]
pub enum HabitatCommand {
    /// Render the habitat snapshot (areas, entities, divergences)
    Snapshot {
        /// SQLite database path
        #[arg(long, default_value = "habitat.db")]
        db_path: String,
    },
    /// Create (or reuse) a coarse-geometry area
    AddArea {
        /// Area name (e.g. "desk")
        name: String,
        /// Grid width in cells
        #[arg(long, default_value_t = 16)]
        grid_w: i64,
        /// Grid height in cells
        #[arg(long, default_value_t = 4)]
        grid_h: i64,
        /// SQLite database path
        #[arg(long, default_value = "habitat.db")]
        db_path: String,
    },
    /// Place (or move) a positioned entity in the model
    Place {
        /// Stable entity id (e.g. a guard pending-entry id or agent id)
        id: String,
        /// Area name to place the entity in
        #[arg(long)]
        area: String,
        /// Entity kind: agent, artifact, pending_guard_action, suspended_runtime, unresolved_decision
        #[arg(long)]
        kind: String,
        /// State string (for agents: working/blocked/idle)
        #[arg(long, default_value = "idle")]
        state: String,
        /// Human-readable label
        #[arg(long, default_value = "")]
        label: String,
        /// Grid x position
        #[arg(long)]
        x: i64,
        /// Grid y position
        #[arg(long)]
        y: i64,
        /// SQLite database path
        #[arg(long, default_value = "habitat.db")]
        db_path: String,
    },
    /// Record a divergence between physical environment and model (exposed, never auto-corrected)
    Divergence {
        /// Area name the divergence concerns
        #[arg(long)]
        area: String,
        /// Description of the discrepancy
        description: String,
        /// SQLite database path
        #[arg(long, default_value = "habitat.db")]
        db_path: String,
    },
    /// Resolve a divergence (the human decided which side is authoritative)
    Resolve {
        /// Divergence id
        id: i64,
        /// SQLite database path
        #[arg(long, default_value = "habitat.db")]
        db_path: String,
    },
    /// Emit the habitat run-state contract (dagr v3 run.json) — the
    /// producer side of the habitat dashboard. Pure projection of the
    /// triage queue, guard pending queue, and theory state-doc staleness;
    /// the renderer (dagr pane, or a future skin) is a pure function of
    /// the emitted file.
    Contract {
        /// Path to the triage queue record (JSONL)
        #[arg(long, default_value = "attempts.jsonl")]
        queue_path: String,
        /// Guard database path (pending actions)
        #[arg(long, default_value = "guard.db")]
        guard_db: String,
        /// Name of the theory state-doc (staleness-checked when indexed)
        #[arg(long, default_value = "theory")]
        theory: String,
        /// SQLite database path for the theory state-doc
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
        /// Write the contract to this path (default: stdout only)
        #[arg(long)]
        out: Option<String>,
    },
}
