#![allow(dead_code)]

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "crabjar",
    about = "CLI for local state-docs management",
    disable_help_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum CliCommand {
    /// Show help as structured JSON
    Help,

    /// Manage state-docs
    State {
        #[command(subcommand)]
        command: StateCommand,
    },

    /// Manage knowledge store
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },

    /// Manage dotfile promotions
    Dotfile {
        #[command(subcommand)]
        command: DotfileCommand,
    },

    /// Show workspace configuration
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },

    /// Guard: pending queue management and provenance verification
    Guard {
        #[command(subcommand)]
        command: GuardCommand,
    },

    /// Execute command with guard + telemetry
    Exec {
        #[arg(long)]
        command: String,

        #[arg(short, long)]
        args: Vec<String>,

        #[arg(short = 'C', long, default_value = "")]
        cwd: String,

        #[arg(short, long)]
        reason: String,

        #[arg(short, long, default_value = "false")]
        dry_run: bool,
    },

    /// Manage bitwarden credentials
    Bitwarden {
        #[command(subcommand)]
        command: BitwardenCommand,
    },

    /// Pre-flight system validation
    Doctor {
        #[command(subcommand)]
        command: DoctorCommand,
    },

    /// Manage inference backend (LM Studio vs Native)
    Backend {
        #[command(subcommand)]
        command: BackendCommand,
    },

    /// Manage tool registry
    Tool {
        #[command(subcommand)]
        command: ToolCommand,
    },

    /// Workspace metrics (tests, modules, LoC, clippy)
    Metrics {
        #[command(subcommand)]
        command: MetricsCommand,
    },

    /// Spatial habitat: positioned computational state (ADR-003)
    Habitat {
        #[command(subcommand)]
        command: HabitatCommand,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum StateCommand {
    /// Index all state-docs into SQLite
    Index {
        /// Path to the state-docs directory
        #[arg(long, default_value = "state-docs")]
        docs_dir: String,
        /// Path to the SQLite database
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Show a state-doc with configurable zoom depth
    Show {
        /// State-doc name
        doc_name: String,
        /// Zoom level: 1=overview, 2=section, 3=paragraph
        #[arg(long, default_value_t = 2)]
        zoom: u8,
    },
    /// Query a state-doc by section or keyword
    Query {
        /// State-doc name
        doc_name: String,
        /// Section name to query
        #[arg(long)]
        section: Option<String>,
        /// Keyword to search
        #[arg(long)]
        keyword: Option<String>,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// List all indexed state-docs
    List {
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Get confidence assessment for a state-doc
    Confidence {
        /// State-doc name
        doc_name: String,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Get annotations for a state-doc
    Annotations {
        /// State-doc name
        doc_name: String,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Get tables extracted from a state-doc
    Tables {
        /// State-doc name
        doc_name: String,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Get code blocks extracted from a state-doc
    CodeBlocks {
        /// State-doc name
        doc_name: String,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Check staleness status of a state-doc (three-tier: stale/expired/moldy)
    Staleness {
        /// State-doc name
        doc_name: String,
        /// SQLite database path
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum KnowledgeCommand {
    /// Index a state-doc
    Index { doc: String },
    /// Sync state-doc annotations into knowledge store
    Sync { doc: String },
    /// Query knowledge entries by tags
    Query {
        #[arg(long)]
        tags: Vec<String>,
    },
    /// Insert a knowledge entry
    Insert {
        #[arg(long)]
        content: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        tags: Vec<String>,
    },
    /// Verify knowledge store integrity
    Verify,
    /// List recent knowledge events
    Events {
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Deactivate a knowledge entry
    Deactivate {
        id: i64,
        #[arg(long)]
        reason: String,
    },
    /// Resolve an annotation and deactivate derived knowledge
    ResolveAnnotation {
        doc: String,
        #[arg(long)]
        annotation_id: String,
        #[arg(long)]
        reason: String,
    },
    /// Promote a quarantined knowledge entry to active
    Promote {
        /// ID of the quarantined entry to promote
        id: i64,
        /// Reason for promotion
        #[arg(long)]
        reason: String,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum DotfileCommand {
    /// Promote a dotfile to AGENTS.md
    Promote { path: String },
}

#[derive(Debug, Subcommand, Clone)]
pub enum WorkspaceCommand {
    /// Show workspace configuration status
    Status,
}

#[derive(Debug, Subcommand, Clone)]
pub enum BitwardenCommand {
    /// Check bitwarden CLI status
    Status,
    /// List bitwarden items
    List {
        #[arg(long)]
        folder: Option<String>,
        #[arg(long)]
        collection: Option<String>,
    },
    /// Get a bitwarden item by ID
    Get {
        #[arg(long)]
        id: String,
    },
    /// Search bitwarden items by name
    Search {
        #[arg(long)]
        query: String,
    },
    /// Generate a password
    Generate {
        #[arg(long, default_value = "32")]
        length: u32,
        #[arg(long, default_value = "true")]
        uppercase: bool,
        #[arg(long, default_value = "true")]
        lowercase: bool,
        #[arg(long, default_value = "true")]
        numbers: bool,
        #[arg(long, default_value = "true")]
        special: bool,
    },
}

/// Inference backend management commands
#[derive(Debug, Subcommand, Clone)]
pub enum BackendCommand {
    /// Set the inference backend (lm-studio or native)
    Set {
        #[arg(short, long)]
        backend: String,
    },
    /// Get the current inference backend
    Get,
}

/// Tool registry management subcommands
#[derive(Debug, Subcommand, Clone)]
pub enum ToolCommand {
    /// List registered tools
    List {
        /// Filter by tool type (command, mcp, llm, vector_store, embedding, rag, skill)
        #[arg(short, long)]
        r#type: Option<String>,
        /// Maximum number of tools to return
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Discover tools from known sources
    Discover {
        /// Source name to use for discovery
        #[arg(short, long, default_value = "cli")]
        source: String,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum GuardCommand {
    /// List pending queue entries
    Queue {
        #[arg(long, default_value = "pending")]
        status: String,
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Approve a pending action
    Approve {
        #[arg(long)]
        action_id: String,
    },
    /// Reject a pending action
    Reject {
        #[arg(long)]
        action_id: String,
        #[arg(long, default_value = "reviewed")]
        reason: String,
    },
    /// List interrupted log entries
    Interrupted {
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Verify provenance for a source event ID
    Provenance {
        #[arg(long)]
        source_event_id: String,
    },
    /// Grant PID trust access
    Grant {
        #[arg(long)]
        pid: i32,
        #[arg(long, default_value = "0")]
        trust_layer: u32,
        #[arg(long, default_value = "false")]
        auto_grant: bool,
    },
    /// Revoke PID trust access
    Revoke {
        #[arg(long)]
        pid: i32,
    },
    /// Show trust resolution chain for recent actions
    Resolution {
        #[arg(long, default_value = "50")]
        limit: usize,
        #[arg(long, help = "Filter by effective trust layer")]
        effective_layer: Option<u32>,
    },
}

/// Doctor subcommands for pre-flight validation
#[derive(Debug, Subcommand, Clone)]
pub enum DoctorCommand {
    /// Run all pre-flight checks
    Check,
}

/// Workspace metrics subcommands
#[derive(Debug, Subcommand, Clone)]
pub enum MetricsCommand {
    /// Run all workspace metrics (tests, modules, LoC, clippy)
    All,
    /// Run just the test count
    Tests,
    /// Run just the module size check (500 LoC rule)
    Modules,
}

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
}

pub mod bitwarden;
pub mod crabjar_config;
pub mod doctor;
pub mod knowledge_store;
pub mod metrics;
pub mod project_loader;

pub use crabjar_config::ProjectConfig;
pub use project_loader::ProjectLoader;

pub fn cli() -> clap::Command {
    Cli::command()
}
