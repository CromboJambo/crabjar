use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

mod manifest;
mod proxy;
mod terminal_relay;
#[cfg(test)]
mod terminal_relay_tests;

use manifest::Manifest;

const MANIFEST_PATH: &str = "manifest.toml";

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
async fn run_supervisor() -> Result<()> {
    let manifest = Manifest::load(MANIFEST_PATH)?;
    let exe = env::current_exe()?;

    let mut handles = Vec::new();
    for vm in manifest.vms {
        let exe = exe.clone();
        handles.push(tokio::spawn(supervise_vm(exe, vm.name)));
    }

    futures_util::future::join_all(handles).await;
    Ok(())
}

/// Keeps one VM's worker process alive, restarting with exponential
/// backoff if it dies. A clean (status-success) exit is treated as
/// intentional and is not restarted — that's how you'd take a single
/// VM's bridge down for maintenance without touching the others.
async fn supervise_vm(exe: PathBuf, name: String) {
    let mut backoff = Duration::from_secs(1);
    loop {
        tracing::info!(vm = %name, "starting worker");
        let status = Command::new(&exe).arg("--worker").arg(&name).status().await;

        match status {
            Ok(s) if s.success() => {
                tracing::info!(vm = %name, "worker exited cleanly, not restarting");
                break;
            }
            Ok(s) => {
                tracing::warn!(vm = %name, code = ?s.code(), "worker exited, restarting")
            }
            Err(e) => {
                tracing::error!(vm = %name, error = %e, "failed to spawn worker")
            }
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
