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

    /// Execute command with guard + telemetry
    Exec {
        #[arg(short, long)]
        command: String,

        #[arg(short, long)]
        args: Vec<String>,

        #[arg(short, long, default_value = "")]
        cwd: String,

        #[arg(short, long)]
        reason: String,

        #[arg(short, long, default_value = "false")]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum StateCommand {
    /// List all state-docs
    List,
    /// Show a state-doc with annotations
    Show { doc: String },
    /// Add a note annotation
    Annotate { doc: String, message: String },
    /// Add a question annotation
    Question { doc: String, message: String },
    /// Resolve an annotation
    Resolve { doc: String, id: String },
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

pub fn cli() -> clap::Command {
    Cli::command()
}
