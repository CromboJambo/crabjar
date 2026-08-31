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

    /// Attempt graph: falsification record and triage queue (ADR-006)
    Attempts {
        #[command(subcommand)]
        command: AttemptsCommand,
    },
}

pub mod cli_commands;

pub use cli_commands::*;

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
