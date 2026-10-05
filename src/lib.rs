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

        /// Run command in isolated Podman container (rootless)
        #[arg(long, default_value = "false")]
        container: bool,
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

    /// Attempt graph: falsification record and triage queue (ADR-006)
    Attempts {
        #[command(subcommand)]
        command: AttemptsCommand,
    },

    /// Semantic decision-making via embedding similarity (Jev/SemIf pattern)
    Decide {
        #[arg(default_value = "evaluate")]
        subcmd: String,
        observation: Option<String>,
        criteria_file: Option<String>,
    },

    /// Slow Friend daemon: tiered attention system (EXPL-005)
    SlowFriend {
        #[command(subcommand)]
        command: SlowFriendCommand,
    },

    /// Learning loop: analyze skill invocations and suggest improvements
    Learn {
        #[arg(default_value = "analyze")]
        subcmd: String,
        #[arg(long, default_value = "3")]
        min_invocations: usize,
    },

    /// Session search: find past sessions by topic or error type
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum SessionCommand {
    /// Search past sessions for relevant context
    Search {
        query: String,
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// Find sessions related to a specific topic
    Related {
        topic: String,
    },
    /// Find past failures of a specific type
    Failures {
        error_type: String,
    },
}

pub mod cli_commands;
pub mod skill_instrumentation;
pub mod learning_loop;
pub mod session_search;
pub mod feedback_integration;
pub mod environment_awareness;

pub use cli_commands::*;

pub mod bitwarden;
pub mod crabjar_config;
pub mod doctor;
pub mod habitat_contract;
pub mod knowledge_store;
pub mod metrics;
pub mod project_loader;

pub use crabjar_config::ProjectConfig;
pub use project_loader::ProjectLoader;

pub fn cli() -> clap::Command {
    Cli::command()
}
