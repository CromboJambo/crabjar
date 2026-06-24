use std::error::Error;
/// Tracing initialization for the host runtime.
///
/// Sets up tracing-subscriber with both console and file output.
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the global tracing subscriber.
///
/// - Console output: filtered by TRACING_LEVEL env var (default: info)
/// - File output: written to data/logs/host.log
/// - JSON formatting for structured logs
pub fn init_tracing(log_dir: &str) -> Result<(), Box<dyn Error>> {
    // Ensure log directory exists
    std::fs::create_dir_all(log_dir).ok();

    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::DAILY,
        log_dir,
        "host.log",
    );

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true);

    let _env_filter =
        EnvFilter::try_from_env("TRACING_LEVEL").unwrap_or_else(|_| EnvFilter::new("info"));

    let filter_layer = EnvFilter::try_from_env("TRACING_FILTER").unwrap_or_else(|_| {
        EnvFilter::new("crabjar_host=debug,crabjar_host_system=debug,crabjar_host_observe=debug")
    });

    Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .try_init()?;

    tracing::info!(%log_dir, "tracing initialized");
    Ok(())
}
