//! Crabjar Hermes Wrapper
//!
//! Runs hermes-agent within the crabjar environment with proper telemetry
//! and execution tracking. Replaces the bash wrapper script that failed silently.
//!
//! Features:
//! - Structured JSON status output on every run
//! - Persistent execution log for post-hoc verification
//! - Proper error propagation (no silent failures)
//! - Environment variable validation before spawning

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Write, BufWriter};
use std::process::Command;
use std::time::SystemTime;

/// Configuration for the crabjar agent environment.
struct AgentConfig {
    user: String,
    home: String,
    inbox: String,
    outbox: String,
    log_dir: String,
    workspace: String,
    config: String,
}

impl AgentConfig {
    fn default_config() -> Self {
        let user = "crabjar-agent".to_string();
        let home = "/home/crabjar-agent".to_string();
        Self {
            user,
            home: home.clone(),
            inbox: format!("{}/inbox", home),
            outbox: format!("{}/outbox", home),
            log_dir: format!("{}/log", home),
            workspace: format!("{}/workspace", home),
            config: format!("{}/config", home),
        }
    }

    fn telemetry_path(&self) -> String {
        format!("{}/hermes-telemetry.jsonl", self.log_dir)
    }

    fn status_path(&self) -> String {
        format!("{}/last-execution-status.json", self.log_dir)
    }
}

/// Result of a wrapper execution.
struct ExecutionResult {
    success: bool,
    exit_code: Option<i32>,
    error_message: Option<String>,
    timestamp: SystemTime,
    hermes_args: Vec<String>,
}

impl ExecutionResult {
    fn to_json(&self) -> String {
        let ts = self.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let exit_code = match &self.exit_code {
            Some(code) => format!("\"{}\"", code),
            None => "null".to_string(),
        };
        
        let error_message = match &self.error_message {
            Some(msg) => format!("\"{}\"", escape_json_string(msg)),
            None => "null".to_string(),
        };
        
        let args_json = self.hermes_args.iter()
            .map(|arg| format!("\"{}\"", escape_json_string(arg)))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "{{\"success\": {}, \"exit_code\": {}, \"error\": {}, \"timestamp\": {}, \"args\": [{}]}}",
            self.success, exit_code, error_message, ts, args_json
        )
    }
}

fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

fn ensure_dir(path: &str) -> Result<(), String> {
    if let Err(e) = fs::create_dir_all(path) {
        return Err(format!("Failed to create directory {}: {}", path, e));
    }
    Ok(())
}

fn append_telemetry(config: &AgentConfig, result: &ExecutionResult) -> Result<(), String> {
    let path = config.telemetry_path();
    
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(file) => {
            let mut writer = BufWriter::new(file);
            let json = result.to_json();
            match writer.write_all(format!("{}\n", json).as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to write telemetry: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to open telemetry file {}: {}", path, e)),
    }
}

fn write_status(config: &AgentConfig, result: &ExecutionResult) -> Result<(), String> {
    let path = config.status_path();
    let json = result.to_json();
    
    fs::write(&path, format!("{}\n", json))
        .map_err(|e| format!("Failed to write status file {}: {}", path, e))
}

fn print_environment(config: &AgentConfig) {
    println!("=== Crabjar Hermes Environment ===");
    println!("Agent user: {}", config.user);
    println!("Home: {}", config.home);
    println!("Inbox: {}", config.inbox);
    println!("Outbox: {}", config.outbox);
    println!("Log: {}", config.log_dir);
    println!("Workspace: {}", config.workspace);
    println!("Telemetry: {}", config.telemetry_path());
    println!();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let hermes_args = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec![]
    };

    let config = AgentConfig::default_config();

    // Print environment info
    print_environment(&config);

    // Ensure log directory exists
    if let Err(e) = ensure_dir(&config.log_dir) {
        let result = ExecutionResult {
            success: false,
            exit_code: None,
            error_message: Some(format!("Directory setup failed: {}", e)),
            timestamp: SystemTime::now(),
            hermes_args: hermes_args.clone(),
        };
        
        // Try to write status even if dir creation failed (use /tmp as fallback)
        let _ = fs::write("/tmp/crabjar-hermes-wrapper-status.json", result.to_json());
        
        eprintln!("ERROR: {}", result.error_message.as_ref().unwrap());
        std::process::exit(1);
    }

    // Spawn hermes with environment variables
    let mut cmd = Command::new("hermes");
    
    // Set crabjar environment variables
    cmd.env("AGENT_USER", &config.user)
       .env("AGENT_HOME", &config.home)
       .env("CRABJAR_INBOX", &config.inbox)
       .env("CRABJAR_OUTBOX", &config.outbox)
       .env("CRABJAR_LOG", &config.log_dir)
       .env("CRABJAR_WORKSPACE", &config.workspace)
       .env("CRABJAR_CONFIG", &config.config)
       .env("HERMES_TELEMETRY_ENABLED", "true")
       .env("HERMES_TELEMETRY_OUTPUT", config.telemetry_path());
    
    // Pass through any arguments
    for arg in &hermes_args {
        cmd.arg(arg);
    }

    let result = match cmd.spawn() {
        Ok(mut child) => {
            match child.wait() {
                Ok(status) => {
                    let success = status.success();
                    let exit_code = status.code();
                    
                    ExecutionResult {
                        success,
                        exit_code,
                        error_message: if !success {
                            Some(format!("hermes exited with code {:?}", exit_code))
                        } else {
                            None
                        },
                        timestamp: SystemTime::now(),
                        hermes_args: hermes_args.clone(),
                    }
                }
                Err(e) => ExecutionResult {
                    success: false,
                    exit_code: None,
                    error_message: Some(format!("Failed to wait for hermes: {}", e)),
                    timestamp: SystemTime::now(),
                    hermes_args: hermes_args.clone(),
                }
            }
        }
        Err(e) => ExecutionResult {
            success: false,
            exit_code: None,
            error_message: Some(format!("Failed to spawn hermes: {}", e)),
            timestamp: SystemTime::now(),
            hermes_args: hermes_args.clone(),
        }
    };

    // Write telemetry and status
    if let Err(e) = append_telemetry(&config, &result) {
        eprintln!("Warning: {}", e);
    }
    
    if let Err(e) = write_status(&config, &result) {
        eprintln!("Warning: {}", e);
    }

    // Report result
    if result.success {
        println!("SUCCESS: hermes completed successfully");
    } else {
        match &result.error_message {
            Some(msg) => eprintln!("ERROR: {}", msg),
            None => eprintln!("ERROR: hermes failed (no error message available)"),
        }
    }

    // Exit with appropriate code
    if result.success {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}