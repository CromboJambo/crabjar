//! Crabjar Worker Agent Binary
//!
//! Registers with crabjar-conductor, polls for tasks, executes them locally.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "crabjar-worker", version, about = "Crabjar fleet worker agent")]
struct Args {
    /// Conductor service URL
    #[arg(long, default_value = "http://localhost:8091")]
    conductor: String,

    /// Worker name (defaults to hostname)
    #[arg(long)]
    name: Option<String>,

    /// Additional capabilities (comma-separated)
    #[arg(long)]
    capabilities: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().init();

    let mut config = crabjar_worker::WorkerConfig::default();
    config.conductor_url = args.conductor;
    if let Some(name) = args.name {
        config.name = name;
    }
    if let Some(caps) = args.capabilities {
        for cap in caps.split(',') {
            config.capabilities.push(cap.trim().to_string());
        }
    }

    crabjar_worker::run(config).await?;
    Ok(())
}
