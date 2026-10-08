//! Crabjar Conductor — Fleet orchestration service.
//!
//! Provides centralized goal management, task decomposition, worker registration,
//! and cross-machine scheduling for home-lab agent fleets.

use anyhow::Result;
use clap::Parser;
use crabjar_conductor::{Conductor, ConductorConfig};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "crabjar-conductor", version, about = "Fleet orchestration service")]
struct Args {
    /// Listen address
    #[arg(long, default_value = "0.0.0.0")]
    addr: String,

    /// Listen port
    #[arg(long, default_value_t = 8091)]
    port: u16,

    /// Pesti-server URL for LLM inference
    #[arg(long, default_value = "http://localhost:8000")]
    pesti_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let config = ConductorConfig {
        listen_addr: args.addr,
        listen_port: args.port,
        pesti_server_url: args.pesti_url,
    };

    let conductor = Conductor::new(config);

    tracing::info!("Starting crabjar-conductor");
    tracing::info!("Listening on {}:{}", config.listen_addr, config.listen_port);

    // TODO: Start HTTP server with API routes
    // For now, just keep the process running to demonstrate the architecture

    Ok(())
}
