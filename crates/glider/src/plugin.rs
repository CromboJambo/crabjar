//! Glider (Conway's Game of Life) as a Hermes plugin — text-mode TUI via stdio JSON-RPC.
//!
//! This module transforms the glider simulation into a plugin that:
//! - Runs in herdr panes (no terminal size queries, no raw mode requirements)
//! - Accepts commands via JSON-RPC over stdin/stdout
//! - Outputs ASCII art to stdout for display

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Write};
use tokio::time::{sleep, Duration};

// ============================================================================
// JSON-RPC Protocol
// ============================================================================

/// Request envelope for glider commands.
#[derive(Debug, Deserialize)]
struct CommandRequest {
    id: u64,
    method: String,
    params: CommandParams,
}

#[derive(Debug, Deserialize)]
struct CommandParams {
    action: GliderAction,
    #[serde(default)]
    value: Option<String>, // e.g., simulation mode
}

/// Available glider commands.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum GliderAction {
    Start,
    Stop,
    Pause,
    Resume,
    SetMode(String), // "sim", "single", "tetris", etc.
    Step,            // single tick
}

/// Response envelope for glider commands.
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
    generation: Option<u64>,
}

// ============================================================================
// Glider State
// ============================================================================

/// Running glider state (shared between command handler and render loop).
struct GliderState {
    running: bool,
    paused: bool,
    mode: String, // "sim", "single", "tetris", etc.
    generation: u64,
}

impl Default for GliderState {
    fn default() -> Self {
        Self {
            running: false,
            paused: true,
            mode: "sim".to_string(),
            generation: 0,
        }
    }
}

// ============================================================================
// Command Handler (JSON-RPC server)
// ============================================================================

async fn handle_commands() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut output = stdout.lock();

    let mut state = GliderState::default();

    loop {
        // Read JSON-RPC request
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break; // EOF
        }

        let request: CommandRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = CommandResponse {
                    id: 0,
                    result: None,
                    error: Some(format!("parse error: {}", e)),
                };
                writeln!(output, "{}", serde_json::to_string(&resp).unwrap())?;
                continue;
            }
        };

        // Handle command
        let result = match request.method.as_str() {
            "glider/start" => {
                state.running = true;
                state.paused = false;
                CommandResult {
                    status: "started".to_string(),
                    message: Some(format!("Glider simulation started (mode: {})", state.mode)),
                    generation: Some(state.generation),
                }
            }
            "glider/stop" => {
                state.running = false;
                state.paused = true;
                CommandResult {
                    status: "stopped".to_string(),
                    message: Some("Glider simulation stopped".to_string()),
                    generation: None,
                }
            }
            "glider/pause" => {
                state.paused = true;
                CommandResult {
                    status: "paused".to_string(),
                    message: Some("Glider simulation paused".to_string()),
                    generation: Some(state.generation),
                }
            }
            "glider/resume" => {
                state.paused = false;
                CommandResult {
                    status: "resumed".to_string(),
                    message: Some("Glider simulation resumed".to_string()),
                    generation: Some(state.generation),
                }
            }
            "glider/set_mode" => {
                if let Some(val) = request.params.value {
                    state.mode = val;
                    CommandResult {
                        status: "mode_set".to_string(),
                        message: Some(format!("Mode set to: {}", state.mode)),
                        generation: Some(state.generation),
                    }
                } else {
                    CommandResult {
                        status: "error".to_string(),
                        message: Some("Missing mode value".to_string()),
                        generation: None,
                    }
                }
            }
            "glider/step" => {
                // Trigger a single tick (would need to signal render loop)
                state.generation += 1;
                CommandResult {
                    status: "stepped".to_string(),
                    message: Some(format!("Advanced to generation {}", state.generation)),
                    generation: Some(state.generation),
                }
            }
            _ => CommandResult {
                status: "error".to_string(),
                message: Some(format!("Unknown method: {}", request.method)),
                generation: None,
            },
        };

        let response = CommandResponse {
            id: request.id,
            result: Some(result),
            error: None,
        };

        writeln!(output, "{}", serde_json::to_string(&response).unwrap())?;
        output.flush()?;
    }

    Ok(())
}

// ============================================================================
// Render Loop (separate task)
// ============================================================================

async fn render_loop(state: &mut GliderState) {
    // Placeholder: in a real implementation, this would:
    // 1. Load glider pattern from crates/glider/src/
    // 2. Run Conway's Game of Life simulation
    // 3. Render ASCII grid to stdout every tick

    let mut tick = 0u64;
    while state.running {
        if !state.paused {
            // Update simulation (placeholder)
            state.generation += 1;
            
            // Render frame (ASCII art)
            println!("\x1b[2J\x1b[H"); // Clear screen
            println!("🧬 Glider Simulation - Generation: {}", state.generation);
            println!("─────────────────────────────────────");
            println!("Mode: {}", state.mode);
            println!("─────────────────────────────────────");
            println!("  ▓▓▓      ← Glider pattern (placeholder)");
            println!("    ▓      ");
            println!("    ▓      ");
            println!("─────────────────────────────────────");
            println!("Controls: q=quit, Space=pause, m=set mode");
        }

        sleep(Duration::from_millis(100)).await; // 10 FPS
    }
}

// ============================================================================
// Main Entry Point (plugin mode)
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--help" {
        println!(
            "crabjar-glider-plugin — Text-mode TUI plugin for Hermes/Herdr\n\
             \n\
             Usage:\n\
               crabjar-glider-plugin [mode]\n\
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
            println!("🧬 Glider plugin started (stdio mode)");
            
            let mut state = GliderState::default();
            
            // Spawn command handler and render loop
            tokio::join!(
                handle_commands(),
                render_loop(&mut state)
            );
        }
        "text" => {
            // Fallback to direct TUI (like the original app)
            println!("🧬 Glider plugin started (text mode - fallback)");
            // TODO: spawn original run_text_mode() here
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}
