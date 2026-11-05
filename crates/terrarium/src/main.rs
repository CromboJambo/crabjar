//! The terrarium CLI — Phase 1 (flat-grid TUI).
//!
//! `terrarium view` renders a ratatui TUI with zones as rows, crabs as single-glyph
//! + behavior verb, and a tidings ticker. Also watches for file changes.

use clap::Parser;
use std::path::PathBuf;

use terrarium::{contract::Run, ui::TerrariumApp};

#[derive(Parser)]
#[command(
    name = "terrarium",
    about = "An isometric hermit-crab skin over the crabjar habitat contract (dagr v3)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand)]
enum Cmd {
    /// Render the simulated habitat from a contract file.
    View {
        /// Path to the dagr v3 contract (default: ./.dagr/run.json).
        #[arg(default_value = ".dagr/run.json")]
        contract: PathBuf,
        /// Path to the taste file (default: built-in taste).
        #[arg(long)]
        behaviors: Option<PathBuf>,
    },
    /// Validate contract and run one simulation tick without TUI.
    Validate {
        /// Path to the dagr v3 contract (default: ./.dagr/run.json).
        #[arg(default_value = ".dagr/run.json")]
        contract: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Cmd::View {
            contract,
            behaviors,
        } => run_view(&contract, behaviors.as_deref()),
        Cmd::Validate { contract } => run_validate(&contract),
    };
    std::process::exit(code);
}

fn run_validate(contract_path: &std::path::Path) -> i32 {
    // Use Run::load() which handles both raw DAGR and crabjar envelope formats
    let run = match Run::load(contract_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    // Validate DAGR version
    if run.dagr != 3 {
        eprintln!("expected dagr version 3, got {}", run.dagr);
        return 1;
    }

    let total_attempts: usize = run.tasks.iter().map(|t| t.attempts.len()).sum();

    println!("OK: valid DAGR v3 habitat contract");
    println!("  version: {}", run.dagr);
    println!(
        "  generated_at: {}",
        run.generated_at.as_deref().unwrap_or("unknown")
    );
    println!("  run_id: {}", run.run.id.as_deref().unwrap_or("unknown"));
    println!("  tasks: {}", run.tasks.len());
    println!("  total_attempts: {}", total_attempts);

    0
}

fn run_view(contract_path: &std::path::Path, behaviors_path: Option<&std::path::Path>) -> i32 {
    let mut app = TerrariumApp::new(
        contract_path.to_path_buf(),
        behaviors_path.map(|p| p.to_path_buf()),
    );

    match app.run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("TUI error: {}", e);
            1
        }
    }
}
