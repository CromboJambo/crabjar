//! CLI tool: training data deduplication engine.
//!
//! Usage: crabjar-train-dedup <command> [options]
//! Commands: process, status, list, stats

use clap::{Parser, Subcommand};
use crabjar_train_dedup::{DedupEngine, EntryStatus};
use std::fs;

#[derive(Parser)]
#[command(name = "crabjar-train-dedup")]
#[command(about = "Training data deduplication engine with occurrence tracking")]
struct Args {
    /// Path to SQLite registry database
    #[arg(
        short,
        long,
        default_value = "/home/crombo/training-data-dump/registry.db"
    )]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a file through the dedup engine
    Process {
        /// Input file to process
        input: String,
        /// Path to cleaned/processed version (for mark_clean)
        #[arg(long)]
        processed: Option<String>,
    },
    /// Show registry statistics
    Stats,
    /// List entries by status
    List {
        /// Filter by status (pending/clean/duplicate/error)
        #[arg(short, long, default_value = "all")]
        status: String,
    },
}

fn main() {
    let args = Args::parse();
    let engine = DedupEngine::new(&args.db).expect("Failed to open registry database");

    match args.command {
        Commands::Process { input, processed } => {
            let content = fs::read_to_string(&input).expect("Failed to read input file");

            let result = engine
                .process_file(&input, &content, None)
                .expect("Failed to process file");

            match result {
                crabjar_train_dedup::ProcessResult::New(entry) => {
                    println!("NEW entry: {}", entry.content_hash);
                    if let Some(proc_path) = processed {
                        engine
                            .mark_clean(&entry.content_hash, &proc_path)
                            .expect("Failed to mark clean");
                        println!("Marked clean → {}", proc_path);
                    }
                }
                crabjar_train_dedup::ProcessResult::Duplicate(entry) => {
                    println!(
                        "DUPLICATE (occurrence {}): {}",
                        entry.occurrences, entry.content_hash
                    );
                }
            }
        }
        Commands::Stats => {
            let stats = engine.stats().expect("Failed to get stats");
            for (key, value) in &stats {
                println!("{}: {}", key, value);
            }
        }
        Commands::List { status } => {
            let target_status = EntryStatus::from(status.as_str());
            let entries = engine
                .query_by_status(target_status.clone())
                .expect("Failed to query entries");

            println!("Entries with status '{}': {}", target_status, entries.len());
            for entry in &entries {
                println!(
                    "  [{}] {} (occurrences: {}, first seen: {})",
                    entry.content_hash.chars().take(8).collect::<String>(),
                    entry.original_path,
                    entry.occurrences,
                    entry.first_seen.format("%Y-%m-%d %H:%M")
                );
            }
        }
    }
}
