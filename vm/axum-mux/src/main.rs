use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::watch;

mod lifecycle;
mod manifest;
mod proxy;
mod review;
mod terminal_relay;
#[cfg(test)]
mod terminal_relay_tests;

use lifecycle::{LifecycleState, routes};
use manifest::Manifest;

const MANIFEST_PATH: &str = "manifest.toml";

/// Shared shutdown signal. Set to `true` when the supervisor receives
/// SIGTERM/SIGINT; all worker supervisors check this before restarting
/// a crashed child and kill their current child immediately.
type ShutdownFlag = watch::Receiver<bool>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    // Terminal relay mode: run only the terminal relay server.
    if let Some(pos) = args.iter().position(|a| a == "--terminal-relay") {
        let port = args
            .get(pos + 1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8082);
        return terminal_relay::serve("0.0.0.0".to_string(), port).await;
    }

    // Live review bridge: run commands in a herdr session and publish
    // the typed event stream to the relay for unintrusive review.
    // `--review <port> <session> <cmd1> [cmd2 ...]`
    if let Some(pos) = args.iter().position(|a| a == "--review") {
        let rest: Vec<String> = args[pos + 1..].to_vec();
        let port = rest
            .first()
            .and_then(|p| p.parse::<u16>().ok())
            .context("--review requires a port (0 = ephemeral)")?;
        let session = rest
            .get(1)
            .cloned()
            .context("--review requires a session name")?;
        let commands: Vec<&str> = rest[2..].iter().map(String::as_str).collect();
        if commands.is_empty() {
            anyhow::bail!("--review requires at least one command");
        }
        return review::run(port, &session, &commands).await;
    }

    // Re-exec mode: this same binary, invoked as a per-VM worker.
    // `--worker <name>` runs just that one VM's bridge, nothing else.
    if let Some(pos) = args.iter().position(|a| a == "--worker") {
        let name = args
            .get(pos + 1)
            .context("--worker requires a VM name")?
            .clone();
        return run_worker(&name).await;
    }

    run_supervisor().await
}

/// Loads the manifest and spawns one OS process per VM, each running
/// this same binary in worker mode. A panic, hang, or segfault in one
/// VM's proxy is contained to that child process — the supervisor and
/// every other VM's worker are unaffected.
///
/// Graceful shutdown: on SIGTERM/SIGINT, kills all child workers, waits
/// for them to exit (with a 10s timeout), then shuts down cleanly.
async fn run_supervisor() -> Result<()> {
    let manifest = Manifest::load(MANIFEST_PATH)?;
    let exe = env::current_exe()?;

    // Start lifecycle API server on a separate port.
    let config = vm_core::manager::ManagerConfig {
        libvirt_uri: "qemu:///system".to_string(),
        default_timeout: std::time::Duration::from_secs(60),
    };
    let manager = vm_core::manager::VmManager::new(config)?;
    let lifecycle_state = LifecycleState::new(Arc::new(manager));
    let lifecycle_app = routes(lifecycle_state.clone());

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8090")
            .await
            .expect("bind lifecycle port");
        tracing::info!("lifecycle API listening on 127.0.0.1:8090");
        axum::serve(listener, lifecycle_app).await.unwrap();
    });

    // Graceful shutdown: listen for SIGTERM/SIGINT and signal all workers.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
            _ = term.recv() => tracing::info!("received SIGTERM, shutting down"),
        }
        let _ = shutdown_tx.send(true);
        Ok::<(), anyhow::Error>(())
    });

    let mut handles = Vec::new();
    for vm in manifest.vms {
        let exe = exe.clone();
        let mut flag = shutdown_rx.clone();
        handles.push(tokio::spawn(async move {
            supervise_vm(exe, vm.name, &mut flag).await;
        }));
    }

    futures_util::future::join_all(handles).await;
    tracing::info!("all workers stopped");
    Ok(())
}

/// Keeps one VM's worker process alive, restarting with exponential
/// backoff if it dies. A clean (status-success) exit is treated as
/// intentional and is not restarted — that's how you'd take a single
/// VM's bridge down for maintenance without touching the others.
///
/// On shutdown signal: kills the current child process and exits without
/// restarting, regardless of exit status.
async fn supervise_vm(exe: PathBuf, name: String, shutdown: &mut ShutdownFlag) {
    let mut backoff = Duration::from_secs(1);
    loop {
        tracing::info!(vm = %name, "starting worker");
        let mut child = match Command::new(&exe).arg("--worker").arg(&name).spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(vm = %name, error = %e, "failed to spawn worker");
                if *shutdown.borrow() {
                    break;
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                continue;
            }
        };

        // Race: wait for child to exit OR shutdown signal.
        let result = tokio::select! {
            status = child.wait() => Some(status),
            _ = shutdown.changed() => {
                tracing::info!(vm = %name, "shutdown requested, killing worker");
                let _ = child.kill().await;
                None
            }
        };

        match result {
            Some(Ok(s)) if s.success() => {
                tracing::info!(vm = %name, "worker exited cleanly, not restarting");
                break;
            }
            Some(Ok(s)) => {
                tracing::warn!(vm = %name, code = ?s.code(), "worker exited, restarting")
            }
            Some(Err(e)) => {
                tracing::error!(vm = %name, error = %e, "failed to wait for worker")
            }
            None => break, // shutdown
        }

        if *shutdown.borrow() {
            break;
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

async fn run_worker(name: &str) -> Result<()> {
    let manifest = Manifest::load(MANIFEST_PATH)?;
    let vm = manifest.find(name)?;
    proxy::serve(vm, manifest.bind_addr).await
}
