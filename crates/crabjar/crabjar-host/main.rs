mod cli;
mod dashboard;
mod tui;

use clap::{Parser, Subcommand};
use crabjar_guard::Scope as GuardScope;
use crabjar_host_agent::AgentLoop;
use crabjar_host_core::{EventBus, HostConfig, PluginRegistry};
use crabjar_host_observe::MetricsCollector;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "crabjar",
    version,
    about = "CrabJar host runtime — Rust-native application host with agent loop"
)]
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
    /// Conversational agent TUI
    Tui {
        /// Objective for the agent (optional)
        #[arg(short, long)]
        objective: Option<String>,
        /// Session ID to resume
        #[arg(long)]
        session: Option<String>,
    },
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
            tracing::info!(
                "starting host runtime (plugin: {}, config: {})",
                plugin,
                config_path
            );

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
                notifications
                    .notify("CrabJar Host", "Runtime started", None)
                    .ok();
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
        Commands::Tui { objective, session } => {
            let obj = objective.as_deref();
            let sid = session.as_deref();
            // Initialize guard DB co-located with data directory
            let guard_db_path = format!("{}/guard.db", cli.data_dir);
            let guard_db = crabjar_guard::GuardDb::open(&guard_db_path).ok();
            // Initialize habitat store co-located with data directory (ADR-003)
            let habitat_db_path = format!("{}/habitat.db", cli.data_dir);
            let habitat_panel = agent_context::habitat::HabitatStore::open(&habitat_db_path)
                .ok()
                .and_then(|store| tui::habitat_panel::HabitatPanel::with_store(store).ok());
            tui::run(obj, sid, guard_db, habitat_panel).await?;
        }
        Commands::PluginList => {
            let plugins = plugin_registry.list().await;
            println!("{}", serde_json::to_string_pretty(&plugins)?);
        }
        Commands::WorkItem { id } => {
            // Placeholder — WorkItem persistence will be added in Phase 2
            tracing::info!(?id, "WorkItem query (placeholder — coming in Phase 2)");
            if let Some(id) = id {
                println!("WorkItem ID: {}", id);
            } else {
                println!("No WorkItem ID specified. Use --id <uuid> to query a specific item.");
            }
        }
        Commands::Tick => {
            let mut loop_engine =
                AgentLoop::new(event_bus, metrics).with_scope(GuardScope::project("host"));
            loop_engine.start("Auto-tick objective");
            let result = loop_engine.tick().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Run { objective } => {
            tracing::info!(objective = %objective, "Starting agent loop");
            let mut loop_engine =
                AgentLoop::new(event_bus, metrics).with_scope(GuardScope::project("host"));
            loop_engine.start(&objective);

            let mut iterations = 0;
            loop {
                iterations += 1;
                let result = loop_engine.tick().await?;
                match &result {
                    crabjar_host_agent::LoopResult::IterationComplete {
                        confidence,
                        tasks_completed,
                        ..
                    } => {
                        tracing::info!(
                            iteration = iterations,
                            confidence = %confidence,
                            tasks = *tasks_completed,
                            "Agent loop iteration complete"
                        );
                    }
                    crabjar_host_agent::LoopResult::Completed { .. } => {
                        tracing::info!(iterations = iterations, "Agent loop completed");
                        println!("Completed after {} iterations", iterations);
                        break;
                    }
                    crabjar_host_agent::LoopResult::Failed { reason, .. } => {
                        tracing::warn!(reason = %reason, "Agent loop failed");
                        println!("Failed: {}", reason);
                        break;
                    }
                }
                if iterations > 200 {
                    tracing::warn!("Max iterations (200) reached without completion");
                    println!("Max iterations reached");
                    break;
                }
            }
        }
    }

    Ok(())
}
