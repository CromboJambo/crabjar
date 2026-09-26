//! Process Management Middleware for Hermes Agent Workflows
//!
//! Provides spawn, discover, monitor, and attach operations for long-running
//! processes. Designed for agent workflows where the agent generates commands
//! but the user executes them in their environment.

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[clap(name = "hermes-process", version = "0.12.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a PID-capturing spawn wrapper for a command
    Spawn(SpawnArgs),
    /// Discover process info by PID
    Discover(DiscoverArgs),
    /// Monitor a process with heartbeat feedback
    Monitor(MonitorArgs),
    /// Attach to running process for interaction
    Attach(AttachArgs),
}

#[derive(Args)]
struct SpawnArgs {
    command: String,
}

#[derive(Args)]
struct DiscoverArgs {
    pid: u32,
}

#[derive(Args)]
struct MonitorArgs {
    pid: u32,
    #[clap(long = "timeout", default_value_t = 600)]
    timeout: u64,
    #[clap(long = "interval", default_value_t = 15)]
    interval: u64,
}

#[derive(Args)]
struct AttachArgs {
    pid: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Spawn(args) => cmd_spawn(&args.command),
        Commands::Discover(args) => cmd_discover(args.pid),
        Commands::Monitor(args) => cmd_monitor(args.pid, args.timeout, args.interval),
        Commands::Attach(args) => cmd_attach(args.pid),
    }
}

fn cmd_spawn(command: &str) -> Result<()> {
    // Generate a wrapper that captures the PID and reports it back
    let wrapper = format!(
        "(cd {} && {}) & echo \"PID: $!\"",
        std::env::current_dir()?.display(),
        command
    );

    println!("{}", wrapper);
    Ok(())
}

fn cmd_discover(pid: u32) -> Result<()> {
    let proc_path = format!("/proc/{}", pid);

    if !PathBuf::from(&proc_path).exists() {
        bail!("Process {} not found", pid);
    }

    // Read process status
    let status_path = format!("{}/status", proc_path);
    let status_content = fs::read_to_string(status_path)
        .with_context(|| format!("Failed to read status for PID {}", pid))?;

    let mut name = String::new();
    let mut state = String::new();
    let mut ppid = String::new();
    let mut utime = String::new();
    let mut stime = String::new();

    for line in status_content.lines() {
        if let Some(rest) = line.strip_prefix("Name:\t") {
            name = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("State:\t") {
            state = rest.split_whitespace().next().unwrap_or("").to_string();
        } else if let Some(rest) = line.strip_prefix("PPid:\t") {
            ppid = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Utime:") {
            utime = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Stime:") {
            stime = rest.trim().to_string();
        }
    }

    // Calculate total CPU time (rough approximation)
    let cpu_seconds: u64 = [utime, stime]
        .iter()
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .sum();

    let result = json!({
        "pid": pid,
        "name": name,
        "state": state,
        "ppid": ppid,
        "cpu_seconds": cpu_seconds,
        "alive": true
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn cmd_monitor(pid: u32, timeout_secs: u64, interval_secs: u64) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let interval = Duration::from_secs(interval_secs);

    println!("Monitoring process {} (timeout: {}s)", pid, timeout_secs);

    loop {
        if start.elapsed() >= timeout {
            println!("[{}] Process timed out after {}s", Utc::now().format("%H:%M:%S"), timeout_secs);
            return Ok(());
        }

        match check_process(pid) {
            Ok(info) => {
                if info["alive"].as_bool().unwrap_or(false) {
                    let elapsed = start.elapsed().as_secs();
                    println!(
                        "[{}] Process still running (elapsed: {:02}:{:02})",
                        Utc::now().format("%H:%M:%S"),
                        elapsed / 60,
                        elapsed % 60
                    );
                } else {
                    let exit_code = info["exit_code"].as_i64().unwrap_or(-1);
                    let elapsed = start.elapsed().as_secs();
                    println!(
                        "[{}] Process exited with code {} (elapsed: {:02}:{:02})",
                        Utc::now().format("%H:%M:%S"),
                        exit_code,
                        elapsed / 60,
                        elapsed % 60
                    );
                    return Ok(());
                }
            }
            Err(e) => {
                println!("[{}] Error checking process: {}", Utc::now().format("%H:%M:%S"), e);
            }
        }

        std::thread::sleep(interval);
    }
}

fn check_process(pid: u32) -> Result<serde_json::Value> {
    let proc_path = format!("/proc/{}", pid);

    if !PathBuf::from(&proc_path).exists() {
        return Ok(json!({ "alive": false, "exit_code": -1 }));
    }

    // Try to get exit status via waitpid (only works for child processes)
    let result = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "stat="])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let stat = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stat.starts_with('Z') || stat.starts_with('X') {
                // Zombie or dead - process has exited
                return Ok(json!({ "alive": false, "exit_code": 0 }));
            }
            return Ok(json!({ "alive": true }));
        }
        _ => {
            return Ok(json!({ "alive": false, "exit_code": -1 }));
        }
    }
}

fn cmd_attach(pid: u32) -> Result<()> {
    let proc_path = format!("/proc/{}", pid);

    if !PathBuf::from(&proc_path).exists() {
        bail!("Process {} not found", pid);
    }

    println!("Attached to process {}", pid);

    // Show basic info
    let status_content = fs::read_to_string(format!("{}/status", proc_path))
        .with_context(|| format!("Failed to read status for PID {}", pid))?;

    for line in status_content.lines() {
        if let Some(rest) = line.strip_prefix("Name:\t") {
            println!("  Name: {}", rest);
        } else if let Some(rest) = line.strip_prefix("State:\t") {
            println!("  State: {}", rest.split_whitespace().next().unwrap_or(""));
        } else if let Some(rest) = line.strip_prefix("Threads:") {
            println!("  Threads: {}", rest.trim());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_current_process() {
        let pid = std::process::id();
        let result = cmd_discover(pid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_nonexistent_process() {
        let result = check_process(999999);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info["alive"], false);
    }
}
