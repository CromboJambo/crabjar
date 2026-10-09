//! Crabjar Conductor — Fleet orchestration service.
//!
//! Provides centralized goal management, task decomposition, worker registration,
//! and cross-machine scheduling for home-lab agent fleets.

use anyhow::Result;
use clap::Parser;
use crabjar_conductor::{api, conductor};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use tracing::info;

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

    /// SQLite database path (default: conductor-state.db in current directory)
    #[arg(long, default_value = "conductor-state.db")]
    db_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default filter if RUST_LOG not set
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    info!("Starting crabjar-conductor");

    let args = Args::parse();

    let config = conductor::ConductorConfig {
        listen_addr: args.addr.clone(),
        listen_port: args.port,
        pesti_server_url: args.pesti_url,
    };

    tracing::info!("Starting crabjar-conductor");
    tracing::info!("Listening on {}:{} ", args.addr, args.port);

    // Create conductor with SQLite state store
    let conductor = conductor::Conductor::with_db(config, &args.db_path)?;

    // Wrap in Arc<Mutex<>> for shared state across API handlers
    use tokio::sync::Mutex;
    let conductor_state: api::ConductorState = Arc::new(Mutex::new(conductor));

    let app = api::routes(conductor_state);

    // Start listening
    info!(addr = %args.addr, port = args.port, "Starting HTTP server");
    let listener = tokio::net::TcpListener::bind(&format!("{}:{}", args.addr, args.port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}