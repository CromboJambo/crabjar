//! File watcher daemon for training data pipeline.
//!
//! Watches the training data dump directory for new files, processes them
//! through the secret stripper and dedup engine, and logs events as DAGR-style JSON.

use chrono::Utc;
use clap::{Parser, Subcommand};
use crabjar_train_dedup::{DedupEngine, EntryStatus};
use crabjar_train_secret_stripper::{strip_with_config, StripConfig};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::mpsc;

/// Error types for the filewatcher daemon.
#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("dedup error: {0}")]
    Dedup(#[from] crabjar_train_dedup::DedupError),
    #[error("strip error: {0}")]
    Strip(#[from] crabjar_train_secret_stripper::StripError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// DAGR-style event contract for training pipeline events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub dagr: i32,
    pub id: String,
    pub kind: String,
    pub source: String,
    pub data: serde_json::Value,
    pub tags: Vec<String>,
}

/// Processing result for a training data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub input_path: String,
    pub output_path: Option<String>,
    pub content_hash: String,
    pub is_duplicate: bool,
    pub occurrence_count: i32,
    pub secrets_found: HashMap<String, usize>,
    pub status: EntryStatus,
    pub processed_at: chrono::DateTime<Utc>,
}

/// The filewatcher daemon.
pub struct FileWatcher {
    watch_dir: PathBuf,
    output_dir: PathBuf,
    db_path: PathBuf,
    dedup_engine: DedupEngine,
}

impl FileWatcher {
    /// Create a new filewatcher daemon.
    pub fn new(
        watch_dir: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
        db_path: impl AsRef<Path>,
    ) -> Result<Self, WatcherError> {
        let dedup_engine = DedupEngine::new(db_path.as_ref())?;

        Ok(Self {
            watch_dir: watch_dir.as_ref().to_path_buf(),
            output_dir: output_dir.as_ref().to_path_buf(),
            db_path: db_path.as_ref().to_path_buf(),
            dedup_engine,
        })
    }

    /// Log a DAGR-style event to stdout as JSON.
    fn log_event(&self, event: PipelineEvent) {
        let json = serde_json::to_string(&event).unwrap_or_default();
        println!("{}", json);
    }

    /// Process a single file through the pipeline.
    pub async fn process_file(&self, path: &Path) -> Result<ProcessResult, WatcherError> {
        let content = fs::read_to_string(path)?;
        let config = StripConfig::default();

        // Step 1: Secret stripping
        let strip_result = strip_with_config(&content, &config)?;
        let cleaned = strip_result.cleaned;
        let secrets_found = strip_result.secrets_found;

        // Step 2: Write cleaned output
        let output_path = self.output_dir.join(format!(
            "{}.cleaned.md",
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        ));
        fs::write(&output_path, &cleaned)?;

        // Step 3: Dedup check
        let result = self.dedup_engine.process_file(
            path.to_string_lossy().to_string(),
            &cleaned,
            Some(secrets_found.keys().cloned().collect()),
        )?;

        match result {
            crabjar_train_dedup::ProcessResult::New(entry) => {
                self.dedup_engine.mark_clean(&entry.content_hash, output_path.to_string_lossy())?;

                let process_result = ProcessResult {
                    input_path: path.to_string_lossy().to_string(),
                    output_path: Some(output_path.to_string_lossy().to_string()),
                    content_hash: entry.content_hash,
                    is_duplicate: false,
                    occurrence_count: 1,
                    secrets_found,
                    status: EntryStatus::Clean,
                    processed_at: Utc::now(),
                };

                // Step 4: Log DAGR event
                let event = PipelineEvent {
                    dagr: 3,
                    id: format!("process-{}", process_result.content_hash),
                    kind: "training_file_processed".to_string(),
                    source: "filewatcher".to_string(),
                    data: serde_json::to_value(&process_result)?,
                    tags: vec!["training-data".to_string(), "new-entry".to_string()],
                };
                self.log_event(event);

                Ok(process_result)
            }
            crabjar_train_dedup::ProcessResult::Duplicate(entry) => {
                let process_result = ProcessResult {
                    input_path: path.to_string_lossy().to_string(),
                    output_path: None,
                    content_hash: entry.content_hash,
                    is_duplicate: true,
                    occurrence_count: entry.occurrences,
                    secrets_found,
                    status: EntryStatus::Duplicate,
                    processed_at: Utc::now(),
                };

                let event = PipelineEvent {
                    dagr: 3,
                    id: format!("dup-{}-{}", process_result.content_hash, Utc::now().timestamp()),
                    kind: "training_file_duplicate".to_string(),
                    source: "filewatcher".to_string(),
                    data: serde_json::to_value(&process_result)?,
                    tags: vec!["training-data".to_string(), "duplicate".to_string()],
                };
                self.log_event(event);

                Ok(process_result)
            }
        }
    }

    /// Start watching the directory for new files.
    pub fn start_watching(&self, tx: mpsc::Sender<PathBuf>) {
        let watch_dir = self.watch_dir.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    if let EventKind::Create(_) = event.kind {
                        for path in event.paths {
                            if path.extension().map_or(false, |ext| ext == "md") {
                                let _ = tx.blocking_send(path);
                            }
                        }
                    }
                }
            },
            notify::Config::default(),
        )
        .expect("Failed to create watcher");

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("Failed to watch directory");

        tracing::info!("Watching {} for new training data files", watch_dir.display());
    }
}

#[derive(Parser)]
#[command(name = "crabjar-train-filewatcher")]
#[command(about = "File watcher daemon for training data pipeline")]
struct Args {
    /// Directory to watch for new training data files
    #[arg(short, long, default_value = "/home/crombo/training-data-dump/")]
    watch_dir: String,

    /// Output directory for cleaned files
    #[arg(
        short,
        long,
        default_value = "/home/crombo/training-data-dump/cleaned/"
    )]
    output_dir: String,

    /// Path to SQLite registry database
    #[arg(short, long, default_value = "/home/crombo/training-data-dump/registry.db")]
    db: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the file watcher daemon
    Watch,
    /// Process a single file through the pipeline
    Process { path: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

    // Ensure output directory exists
    fs::create_dir_all(&args.output_dir)?;

    let watcher = FileWatcher::new(&args.watch_dir, &args.output_dir, &args.db)?;

    match args.command {
        Some(Commands::Watch) => {
            let (tx, mut rx) = mpsc::channel::<PathBuf>(100);
            watcher.start_watching(tx);

            while let Some(path) = rx.recv().await {
                tracing::info!("New file detected: {}", path.display());
                match watcher.process_file(&path).await {
                    Ok(result) => {
                        if result.is_duplicate {
                            tracing::info!(
                                "Duplicate (occurrence {}): {}",
                                result.occurrence_count,
                                result.content_hash.chars().take(8).collect::<String>()
                            );
                        } else {
                            tracing::info!(
                                "New entry: {}",
                                result.content_hash.chars().take(8).collect::<String>()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process {}: {}", path.display(), e);
                    }
                }
            }
        }
        Some(Commands::Process { path }) => {
            let path = PathBuf::from(path);
            if !path.exists() {
                return Err(format!("File not found: {}", path.display()).into());
            }

            tracing::info!("Processing file: {}", path.display());
            let result = watcher.process_file(&path).await?;

            println!(
                "{} entry (occurrence {}): {}",
                if result.is_duplicate { "Duplicate" } else { "New" },
                result.occurrence_count,
                result.content_hash.chars().take(8).collect::<String>()
            );

            if !result.secrets_found.is_empty() {
                println!("Secrets found:");
                for (secret_type, count) in &result.secrets_found {
                    println!("  {}: {}", secret_type, count);
                }
            }
        }
        None => {
            // No command specified — show help
            use clap::CommandFactory;
            Args::command().print_help()?;
        }
    }

    Ok(())
}
