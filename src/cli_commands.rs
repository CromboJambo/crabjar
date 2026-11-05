//! CLI subcommand enums (clap `Subcommand` definitions).
//!
//! Split out of `lib.rs` by the 500 LoC module gate: the root `Cli` and
//! `CliCommand` stay in `lib.rs`; every subcommand tree lives here.

use clap::Subcommand;

// Re-export split modules so `pub use cli_commands::*` still works
pub use crate::cli_habitat_commands::HabitatCommand;
pub use crate::cli_tool_commands::{ToolCacheCommand, ToolCommand};

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
    /// Check drift status of a state-doc (coasting vs resisting)
    Drift {
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

/// MCP client subcommands — connect to external MCP servers and call their tools
#[derive(Debug, Subcommand, Clone)]
pub enum McpCommand {
    /// List tools available on an external MCP server (stdio transport)
    Tools {
        /// Program to spawn as the MCP server
        program: String,
        /// Arguments to pass to the MCP server
        #[arg(num_args(0..))]
        args: Vec<String>,
    },
    /// Call a tool on an external MCP server (stdio transport)
    Call {
        /// Program to spawn as the MCP server
        program: String,
        /// Arguments to pass to the MCP server
        #[arg(num_args(0..))]
        server_args: Vec<String>,
        /// Tool name to call
        tool: String,
        /// JSON arguments for the tool (default: {})
        #[arg(long, default_value = "{}")]
        params: String,
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

/// Slow Friend daemon subcommands (EXPL-005)
#[derive(Debug, Subcommand, Clone)]
pub enum SlowFriendCommand {
    /// Start the Slow Friend daemon loop (tiered attention system)
    Start {
        /// Tick interval in seconds (default: 60)
        #[arg(long, default_value_t = 60)]
        tick_interval: u64,
        /// Sleep duration after quiet period in seconds (default: 300)
        #[arg(long, default_value_t = 300)]
        sleep_after_quiet: u64,
    },
    /// Run a single sensor check and report (non-daemon mode)
    Check,
    /// Show autonomy tracker status for action classes
    Autonomy {
        /// Action class to query (disk-cleanup, restart-service, etc.)
        #[arg(long)]
        class: Option<String>,
    },
}

/// Hermes-compatible memory tool subcommands
#[derive(Debug, Subcommand, Clone)]
pub enum MemoryCommand {
    /// Add a new memory entry
    Add {
        /// Target store: "memory" (agent notes) or "user" (user profile)
        #[arg(long, default_value = "memory")]
        target: String,
        /// Entry content
        content: String,
    },
    /// Patch an existing memory entry
    Patch {
        /// Target store: "memory" or "user"
        #[arg(long, default_value = "memory")]
        target: String,
        /// Substring to find (must be unique)
        #[arg(long)]
        old_text: String,
        /// Replacement text
        #[arg(long)]
        content: String,
    },
    /// Remove a memory entry by substring match
    Remove {
        /// Target store: "memory" or "user"
        #[arg(long, default_value = "memory")]
        target: String,
        /// Substring to find (must be unique)
        #[arg(long)]
        old_text: String,
    },
    /// List current memory entries with capacity info
    List {
        /// Target store: "memory" or "user"
        #[arg(long, default_value = "memory")]
        target: String,
    },
}

/// Attempt graph subcommands (ADR-006)
#[derive(Debug, Subcommand, Clone)]
pub enum AttemptsCommand {
    /// Maintenance debt dashboard: unjudged count, oldest age, broken
    /// conditions, queue budget usage, theory staleness
    Status {
        /// Path to the triage queue record (JSONL)
        #[arg(long, default_value = "attempts.jsonl")]
        queue_path: String,
        /// Name of the theory state-doc (staleness-checked when indexed)
        #[arg(long, default_value = "theory")]
        theory: String,
        /// SQLite database path for the theory state-doc
        #[arg(long, default_value = "state-docs.db")]
        db_path: String,
    },
    /// Rewind an attempt commit (fine tier: git revert; user-initiated)
    Rewind {
        /// The commit to revert (the attempt's own commit)
        commit: String,
        /// The work tree the attempt lives in
        #[arg(long, default_value = ".")]
        workdir: String,
        /// The trunk line the commit must sit on (default: current HEAD)
        #[arg(long)]
        trunk: Option<String>,
        /// Pre-flight only: report whether the revert is clean, change nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Submit a delegated subagent task (ADR-012). Enqueues a non-blocking
    /// background work item for an available worker. The primary coding flow
    /// continues uninterrupted; results land in the triage queue.
    Submit {
        /// Task description / intent
        #[arg(long)]
        intent: String,
        /// Command to execute (for command-based tasks)
        #[arg(long)]
        command: Option<String>,
        /// Path to the triage queue record (JSONL)
        #[arg(long, default_value = "attempts.jsonl")]
        queue_path: String,
    },
}
