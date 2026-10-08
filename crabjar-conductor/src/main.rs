//! Crabjar Conductor — Fleet orchestration service.
//!
//! Provides centralized goal management, task decomposition, worker registration,
//! and cross-machine scheduling for home-lab agent fleets.

use anyhow::Result;
use clap::Parser;
use crabjar_conductor::{api, conductor};
use std::sync::Arc;
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

    let config = conductor::ConductorConfig {
        listen_addr: args.addr.clone(),
        listen_port: args.port,
        pesti_server_url: args.pesti_url,
    };

    tracing::info!("Starting crabjar-conductor");
    tracing::info!("Listening on {}:{} ", args.addr, args.port);

    // Create conductor with in-memory state store for now
    let conductor = Arc::new(conductor::Conductor::new(config)?);

    // Build the HTTP API router
    let app = api::routes();

    // Start listening
    let listener = tokio::net::TcpListener::bind(&format!("{}:{}", args.addr, args.port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}