//! Terrarium as a Hermes plugin — text-mode TUI via stdio JSON-RPC.
//!
//! This module transforms the terrarium into a plugin that:
//! - Runs in herdr panes (no terminal size queries, no raw mode requirements)
//! - Accepts commands via JSON-RPC over stdin/stdout
//! - Outputs ASCII art/emoji to stdout for display
//! - Supports pause/resume/speed controls

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Write};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

// ============================================================================
// JSON-RPC Protocol
// ============================================================================

/// Request envelope for terrarium commands.
#[derive(Debug, Deserialize)]
struct CommandRequest {
    id: u64,
    method: String,
    params: CommandParams,
}

#[derive(Debug, Deserialize)]
struct CommandParams {
    action: TerrariumAction,
    #[serde(default)]
    value: Option<String>, // e.g., speed multiplier
}

/// Available terrarium commands.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum TerrariumAction {
    Start,
    Stop,
    Pause,
    Resume,
    SetSpeed(String), // "0.5", "1.0", "10.0"
    Step,            // single tick
}

/// Response envelope for terrarium commands.
#[derive(Debug, Serialize)]
struct CommandResponse {
    id: u64,
    result: Option<CommandResult>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CommandResult {
    status: String,
    message: Option<String>,
    crabs_count: Option<usize>,
}

// ============================================================================
// Terrarium State (shared via Arc<Mutex>)
// ============================================================================

/// Running terrarium state (shared between command handler and render loop).
struct TerrariumState {
    running: bool,
    paused: bool,
    speed_multiplier: f64,
    crabs_count: usize,
}

impl Default for TerrariumState {
    fn default() -> Self {
        Self {
            running: false,
            paused: true,
            speed_multiplier: 1.0,
            crabs_count: 0,
        }
    }
}

// ============================================================================
// Command Handler (JSON-RPC server)
// ============================================================================

async fn handle_commands(state: &Mutex<TerrariumState>) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut output = stdout.lock();

    eprintln!("DEBUG: handle_commands STARTED");

    loop {
        // Read JSON-RPC request
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("DEBUG: EOF received, breaking");
                break; // EOF
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("DEBUG: read error: {}", e);
                continue;
            }
        }

        eprintln!("DEBUG: received command: {}", line.trim());

        let request: CommandRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = CommandResponse {
                    id: 0,
                    result: None,
                    error: Some(format!("parse error: {}", e)),
                };
                writeln!(output, "{}", serde_json::to_string(&resp).unwrap()).unwrap();
                output.flush().unwrap();
                continue;
            }
        };

        // Handle command - lock state mutably
        let result = {
            let mut state_guard = state.lock().await;
            match request.method.as_str() {
                "terrarium/start" => {
                    state_guard.running = true;
                    state_guard.paused = false;
                    CommandResult {
                        status: "started".to_string(),
                        message: Some("Terrarium started".to_string()),
                        crabs_count: Some(state_guard.crabs_count),
                    }
                }
                "terrarium/stop" => {
                    state_guard.running = false;
                    state_guard.paused = true;
                    CommandResult {
                        status: "stopped".to_string(),
                        message: Some("Terrarium stopped".to_string()),
                        crabs_count: None,
                    }
                }
                "terrarium/pause" => {
                    state_guard.paused = true;
                    CommandResult {
                        status: "paused".to_string(),
                        message: Some("Terrarium paused".to_string()),
                        crabs_count: Some(state_guard.crabs_count),
                    }
                }
                "terrarium/resume" => {
                    state_guard.paused = false;
                    CommandResult {
                        status: "resumed".to_string(),
                        message: Some("Terrarium resumed".to_string()),
                        crabs_count: Some(state_guard.crabs_count),
                    }
                }
                "terrarium/set_speed" => {
                    if let Some(val) = request.params.value {
                        state_guard.speed_multiplier = val.parse().unwrap_or(1.0);
                        CommandResult {
                            status: "speed_set".to_string(),
                            message: Some(format!("Speed set to {}x", state_guard.speed_multiplier)),
                            crabs_count: Some(state_guard.crabs_count),
                        }
                    } else {
                        CommandResult {
                            status: "error".to_string(),
                            message: Some("Missing speed value".to_string()),
                            crabs_count: None,
                        }
                    }
                }
                "terrarium/step" => {
                    // Trigger a single tick (would need to signal render loop)
                    CommandResult {
                        status: "stepped".to_string(),
                        message: Some("Advanced one tick".to_string()),
                        crabs_count: Some(state_guard.crabs_count),
                    }
                }
                _ => CommandResult {
                    status: "error".to_string(),
                    message: Some(format!("Unknown method: {}", request.method)),
                    crabs_count: None,
                },
            }
        };

        let response = CommandResponse {
            id: request.id,
            result: Some(result),
            error: None,
        };

        writeln!(output, "{}", serde_json::to_string(&response).unwrap())?;
        output.flush()?;
    }

    eprintln!("DEBUG: handle_commands EXITED");
    Ok(())
}

// ============================================================================
// Render Loop (separate task)
// ============================================================================

async fn render_loop(state: &Mutex<TerrariumState>) {
    // Placeholder: in a real implementation, this would:
    // 1. Spawn the terrarium world logic (from apps/terrarium/src/world.rs)
    // 2. Render ASCII art/emoji to stdout every tick
    // 3. Respond to state changes from command handler

    eprintln!("DEBUG: render_loop STARTED");
    
    let mut tick = 0u64;
    loop {
        let running = {
            let state_guard = state.lock().await;
            state_guard.running
        };
        
        if !running {
            break; // Exit when running = false
        }
        
        let paused = {
            let state_guard = state.lock().await;
            state_guard.paused
        };
        
        let speed = {
            let state_guard = state.lock().await;
            state_guard.speed_multiplier
        };

        if !paused {
            // Update world logic here
            tick += 1;
            
            // Render frame (placeholder) - use eprintln for debugging
            eprintln!("DEBUG: render_loop tick={} speed={}", tick, speed);
            print!("\x1b[2J\x1b[H"); // Clear screen
            print!("🦀 Terrarium - Tick: {} | Speed: {}x", tick, speed);
            print!("─────────────────────────────────────");
            print!("🐍 Snake moving... (placeholder)");
            print!("─────────────────────────────────────");
            print!("Controls: q=quit, Space=pause, +=speed, -=slow");
            std::io::stdout().flush().unwrap(); // Force flush
        }

        sleep(Duration::from_millis(50)).await; // 20 FPS
    }
    
    eprintln!("DEBUG: render_loop EXITED");
}

// ============================================================================
// Main Entry Point (plugin mode)
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--help" {
        println!(
            "crabjar-terrarium-plugin — Text-mode TUI plugin for Hermes/Herdr\n\
             \n\
             Usage:\n\
               crabjar-terrarium-plugin [mode]\n\
             \n\
             Modes:\n\
               stdio   JSON-RPC over stdin/stdout (default in herdr)\n\
               text    Direct TUI output (for testing)\n\
         "
        );
        return;
    }

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("stdio");

    match mode {
        "stdio" => {
            println!("🦀 Terrarium plugin started (stdio mode)");
            
            let state = Mutex::new(TerrariumState::default());
            
            // Spawn command handler and render loop with shared state
            tokio::join!(
                handle_commands(&state),
                render_loop(&state)
            );
        }
        "text" => {
            // Fallback to direct TUI (like the original app)
            println!("🦀 Terrarium plugin started (text mode - fallback)");
            // TODO: spawn original run_text_mode() here
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}
