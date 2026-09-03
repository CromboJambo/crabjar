//! Main entry point for CrabJar isometric terrarium with DAGR integration.
//!
//! Combines:
//! - Isometric rendering (render_isometric)
//! - Game state management (render_isometric)
//! - Agent command handling (command_handler) — the glass-clean seam that
//!   mutates `GameWorld` and returns side-effect events as data
//! - DAGR event production (dagr_feed) — emitted to stderr so stdout stays a
//!   clean JSON-RPC channel

mod render_isometric;
mod dagr_feed;
mod command_handler;

use crate::command_handler::{execute_command, CommandResult};
use crate::dagr_feed::{build_event, emit_event, AgentCommand, EventSource, EventType};
use crate::render_isometric::{generate_world, GameWorld, render_world};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};

// ============================================================================
// Command params (control + agent gameplay)
// ============================================================================

/// Command params. Control commands use `action`/`value`; agent gameplay
/// commands carry a full `AgentCommand` payload (move / build / interact).
#[derive(Debug, Deserialize)]
struct CommandParams {
    #[serde(default)]
    action: Option<TerrariumAction>,
    #[serde(default)]
    value: Option<String>,
    /// Agent-driven gameplay command (the DAGR-feed seam).
    #[serde(default)]
    command: Option<AgentCommand>,
}

/// Available terrarium control commands.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum TerrariumAction {
    Start,
    Stop,
    Pause,
    Resume,
    SetSpeed(String), // "0.5", "1.0", "10.0"
    Step,             // single tick
}

// ============================================================================
// Shared plugin state
// ============================================================================

struct PluginState {
    world: Mutex<GameWorld>,
}

impl PluginState {
    fn new(world: GameWorld) -> Self {
        Self {
            world: Mutex::new(world),
        }
    }
}

// ============================================================================
// JSON-RPC envelopes
// ============================================================================

#[derive(Debug, Deserialize)]
struct CommandRequest {
    id: u64,
    method: String,
    params: Option<CommandParams>,
}

#[derive(Debug, Serialize)]
struct CommandResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--help" {
        println!(
            "crabjar-terrarium — Isometric habitat with DAGR integration\n\
             \n\
             Usage:\n\
               crabjar-terrarium [mode]\n\
             \n\
             Modes:\n\
               stdio   JSON-RPC over stdin/stdout (default in herdr)\n\
               demo    Standalone isometric rendering\n"
        );
        return;
    }

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("stdio");

    match mode {
        "stdio" => {
            eprintln!("crabjar-terrarium (JSON-RPC mode)");

            // Generate initial world and share it between the command handler
            // and the render loop.
            let state = std::sync::Arc::new(PluginState::new(generate_world(20, 15)));

            // Run both the command handler and the renderer in parallel. The
            // first to finish (EOF on stdin) ends the process.
            let state_for_commands = state.clone();
            let state_for_render = state.clone();
            tokio::select! {
                _ = handle_json_rpc_commands(state_for_commands) => {},
                _ = run_isometric_world_with_state(state_for_render) => {},
            }
        }
        "demo" => {
            eprintln!("crabjar-terrarium (Demo mode)");

            let mut world = generate_world(20, 15);
            world.tick = 1;
            world.paused = false;

            run_isometric_world(world).await;
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}

// ============================================================================
// JSON-RPC Command Handler
// ============================================================================

async fn handle_json_rpc_commands(state: std::sync::Arc<PluginState>) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut output = stdout.lock();

    loop {
        // Read a JSON-RPC request line.
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {}", e);
                continue;
            }
        }

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

        // Handle the command — lock the world, mutate it, emit DAGR events.
        let result = {
            let mut world_guard = state.world.lock().await;
            handle_command(&mut world_guard, &request)
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

/// Dispatch a single command against the world. Returns the JSON-RPC result
/// and emits any side-effect DAGR events to stderr (the feed) as it goes.
fn handle_command(world: &mut GameWorld, request: &CommandRequest) -> serde_json::Value {
    // Agent gameplay commands take precedence when present: they go through
    // the tested command_handler seam, which mutates the world and returns
    // the events that describe what changed.
    if let Some(agent_cmd) = request.params.as_ref().and_then(|p| p.command.clone()) {
        let outcome = execute_command(world, agent_cmd);

        // Emit each side-effect event to the DAGR feed (stderr).
        for event_type in &outcome.events {
            emit_event(&build_event(event_type.clone(), EventSource::Agent, world.tick));
        }

        // Also record the command itself as an AgentCommand event.
        let agent_id = "agent".to_string();
        let cmd_name = match &outcome.result {
            CommandResult { success: true, .. } => "ok",
            _ => "fail",
        };
        emit_event(&build_event(
            EventType::AgentCommand {
                agent_id: agent_id.clone(),
                command: cmd_name.to_string(),
                parameters: serde_json::json!({}),
                success: outcome.result.success,
            },
            EventSource::Agent,
            world.tick,
        ));

        return serde_json::to_value(&outcome.result).unwrap();
    }

    // Control commands (pause / resume / speed / start / stop / step).
    let method = request.method.as_str();
    let result: CommandResult = match method {
        "terrarium/start" => {
            world.paused = false;
            world.tick = 1;
            CommandResult::success("Terrarium started")
        }
        "terrarium/stop" => {
            world.paused = true;
            CommandResult::success("Terrarium stopped")
        }
        "terrarium/pause" => {
            world.paused = true;
            CommandResult::success("Paused")
        }
        "terrarium/resume" => {
            world.paused = false;
            CommandResult::success("Resumed")
        }
        "terrarium/set_speed" => {
            let value = request.params.as_ref().and_then(|p| p.value.clone());
            match value.and_then(|v| v.parse::<f32>().ok()) {
                Some(speed) => {
                    world.speed = speed.max(0.1);
                    CommandResult::success(format!("Speed set to {}x", world.speed))
                }
                None => CommandResult::failure("Missing or invalid speed value"),
            }
        }
        "terrarium/step" => {
            if !world.paused {
                crate::render_isometric::step_world(world, 0.033);
            }
            CommandResult::success("Advanced one tick")
        }
        other => CommandResult::failure(format!("Unknown method: {}", other)),
    };

    // Record the control command as an AgentCommand event on the feed.
    emit_event(&build_event(
        EventType::AgentCommand {
            agent_id: "user".to_string(),
            command: method.to_string(),
            parameters: serde_json::json!({}),
            success: result.success,
        },
        EventSource::User,
        world.tick,
    ));

    serde_json::to_value(&result).unwrap()
}

// ============================================================================
// Render loop (steps the *shared* world so agent mutations are visible)
// ============================================================================

/// Minimum distance (in grid cells) an entity must move before we emit another
/// EntityMove event. Throttles the feed so sub-cell gliding doesn't flood it.
const MOVE_EVENT_THRESHOLD: f32 = 0.5;

async fn run_isometric_world_with_state(state: std::sync::Arc<PluginState>) {
    // Track each entity's last reported position for move-event throttling.
    let mut last_reported: HashMap<String, (f32, f32)> = {
        let guard = state.world.lock().await;
        guard
            .entities
            .iter()
            .map(|e| (e.id.clone(), (e.x, e.y)))
            .collect()
    };

    loop {
        // Step the shared world directly — no local copy to drift from. In
        // stdio mode this is headless: we advance simulation time and emit
        // DAGR events, but do NOT render (stdout is reserved for JSON-RPC).
        {
            let mut guard = state.world.lock().await;
            if !guard.paused {
                crate::render_isometric::step_world(&mut guard, 0.033);
                // Emit throttled EntityMove events for entities that moved a
                // meaningful distance since their last report.
                for entity in &guard.entities {
                    let (lx, ly) = last_reported
                        .get(&entity.id)
                        .copied()
                        .unwrap_or((entity.x, entity.y));
                    let dx = entity.x - lx;
                    let dy = entity.y - ly;
                    if dx * dx + dy * dy >= MOVE_EVENT_THRESHOLD * MOVE_EVENT_THRESHOLD {
                        emit_event(&build_event(
                            EventType::EntityMove {
                                entity_id: entity.id.clone(),
                                from_x: lx,
                                from_y: ly,
                                to_x: entity.x,
                                to_y: entity.y,
                                direction: None,
                            },
                            EventSource::Simulation,
                            guard.tick,
                        ));
                        last_reported.insert(entity.id.clone(), (entity.x, entity.y));
                    }
                }
            }
        }

        let speed = {
            let guard = state.world.lock().await;
            guard.speed
        };
        let sleep_ms = (50.0 / speed).max(10.0) as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
    }
}

/// Standalone render loop (demo mode) — no JSON-RPC, no shared state.
async fn run_isometric_world(mut world: GameWorld) {
    let mut last_reported: HashMap<String, (f32, f32)> =
        world.entities.iter().map(|e| (e.id.clone(), (e.x, e.y))).collect();

    loop {
        if !world.paused {
            crate::render_isometric::step_world(&mut world, 0.033);

            for entity in &world.entities {
                let (lx, ly) = last_reported
                    .get(&entity.id)
                    .copied()
                    .unwrap_or((entity.x, entity.y));
                let dx = entity.x - lx;
                let dy = entity.y - ly;
                if dx * dx + dy * dy >= MOVE_EVENT_THRESHOLD * MOVE_EVENT_THRESHOLD {
                    emit_event(&build_event(
                        EventType::EntityMove {
                            entity_id: entity.id.clone(),
                            from_x: lx,
                            from_y: ly,
                            to_x: entity.x,
                            to_y: entity.y,
                            direction: None,
                        },
                        EventSource::Simulation,
                        world.tick,
                    ));
                    last_reported.insert(entity.id.clone(), (entity.x, entity.y));
                }
            }

            render_world(&world);
        } else {
            render_world(&world);
        }

        let sleep_ms = (50.0 / world.speed).max(10.0) as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
    }
}
