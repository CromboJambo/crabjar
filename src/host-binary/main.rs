mod cli;
mod dashboard;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use crabjar_host_core::{HostConfig, EventBus, PluginRegistry};
use crabjar_host_observe::MetricsCollector;
use crabjar_host_agent::AgentLoop;

#[derive(Parser, Debug)]
#[command(name = "crabjar", version, about = "CrabJar host runtime — Rust-native application host with agent loop")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Data directory
    #[arg(long, global = true, default_value = "~/.config/crabjar-host/data")]
    data_dir: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the host runtime
    Start {
        /// Plugin to load (e.g., "teams")
        #[arg(short, long, default_value = "teams")]
        plugin: String,
    },
    /// Show runtime status
    Status,
    /// Run the Ratatui Mission Control dashboard
    Dashboard,
    /// List registered plugins
    PluginList,
    /// Show WorkItem state
    WorkItem {
        /// WorkItem ID
        #[arg(long)]
        id: Option<String>,
    },
    /// Agent loop — run one tick
    Tick,
    /// Agent loop — run until completion
    Run {
        /// Objective for the agent
        objective: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    let log_dir = format!("{}/logs", cli.data_dir);
    std::fs::create_dir_all(&log_dir).ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .init();
    let config_path = cli.config.unwrap_or_else(|| {
        let base = dirs::config_dir()
            .map(|d| d.join("crabjar-host").join("config.toml"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/crabjar-host/config.toml"));
        base.to_string_lossy().into_owned()
    });

    let config = HostConfig::load_or_default(&config_path)?;

    // Initialize subsystems
    let event_bus = Arc::new(EventBus::new(1024));
    let metrics = MetricsCollector::new();
    let plugin_registry = Arc::new(PluginRegistry::new());

    match cli.command {
        Commands::Start { plugin } => {
            tracing::info!("starting host runtime (plugin: {}, config: {})", plugin, config_path);

            // Register the requested plugin
            let teams_plugin = Box::new(crabjar_app_teams::TeamsPlugin::new());
            plugin_registry.register(teams_plugin).await?;

            // Initialize tray
            let mut tray = crabjar_host_system::SystemTray::new(event_bus.clone());
            if config.tray.enabled {
                tray.show().await.ok();
            }

            // Initialize notifications
            let notifications = crabjar_host_system::NotificationService::new(event_bus.clone());
            if config.notifications.enabled {
                notifications.notify("CrabJar Host", "Runtime started", None).ok();
            }

            tracing::info!("host runtime running — press Ctrl+C to stop");

            // Keep running (in practice, this would be an async select loop)
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
        Commands::Status => {
            let plugins = plugin_registry.list().await;
            let metrics_snap = metrics.snapshot();
            let status = serde_json::json!({
                "plugins": plugins,
                "metrics": metrics_snap,
                "config_path": config_path,
            });
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        Commands::Dashboard => {
            dashboard::run(event_bus, metrics, plugin_registry).await?;
        }
        Commands::PluginList => {
            let plugins = plugin_registry.list().await;
            println!("{}", serde_json::to_string_pretty(&plugins)?);
        }
        Commands::WorkItem { id } => {
            // Placeholder — WorkItem persistence will be added in Phase 2
            println!("WorkItem query (placeholder — coming in Phase 2)");
            if let Some(id) = id {
                println!("  ID: {}", id);
            }
        }
        Commands::Tick => {
            let mut loop_engine = AgentLoop::new(event_bus, metrics);
            loop_engine.start("Auto-tick objective");
            let result = loop_engine.tick().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Run { objective } => {
            let mut loop_engine = AgentLoop::new(event_bus, metrics);
            loop_engine.start(&objective);
            println!("Running agent loop for: {}", objective);

            let mut iterations = 0;
            loop {
                iterations += 1;
                let result = loop_engine.tick().await?;
                match &result {
                    crabjar_host_agent::LoopResult::IterationComplete { confidence, tasks_completed, .. } => {
                        println!("  Iteration {}: confidence={:.0}%, tasks={}/{}", iterations, confidence * 100.0, tasks_completed, tasks_completed);
                    }
                    crabjar_host_agent::LoopResult::Completed { .. } => {
                        println!("  Completed after {} iterations", iterations);
                        break;
                    }
                    crabjar_host_agent::LoopResult::Failed { reason, .. } => {
                        println!("  Failed: {}", reason);
                        break;
                    }
                }
                if iterations > 200 {
                    println!("  Max iterations reached");
                    break;
                }
            }
        }
    }

    Ok(())
}
