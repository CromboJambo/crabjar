//! Terrarium as a Hermes plugin — JSON-RPC over stdio.
//! Self-contained world with actual entity positions updated each tick.
//! Publishes live entity state to orchestrator SSE channel for crabjar-gpui.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Write};
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

// ============================================================================
// JSON-RPC Protocol
// ============================================================================

#[derive(Debug, Deserialize)]
struct CommandRequest {
    id: u64,
    method: String,
    params: Option<CommandParams>,
}

#[derive(Debug, Deserialize)]
struct CommandParams {
    action: TerrariumAction,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum TerrariumAction {
    Start,
    Stop,
    Pause,
    Resume,
    SetSpeed(String),
    Step,
}

#[derive(Debug, Serialize)]
struct CommandResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

// ============================================================================
// World State — actual entity positions that get updated each tick
// ============================================================================

#[derive(Debug, Clone)]
struct Entity {
    id: String,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    symbol: &'static str,
    color: &'static str,
}

struct WorldState {
    entities: Vec<Entity>,
    tick: u64,
    paused: bool,
    speed: f32,
    width: i32,
    height: i32,
}

fn create_world() -> WorldState {
    let entities = vec![
        Entity {
            id: "crab_1".to_string(),
            x: 2.0, y: 3.0, z: 0.0,
            vx: 1.5, vy: 0.8,
            symbol: "🦀", color: "#ff6b6b",
        },
        Entity {
            id: "crab_2".to_string(),
            x: 5.0, y: 1.0, z: 0.0,
            vx: -0.8, vy: 1.2,
            symbol: "🦀", color: "#4ecdc4",
        },
        Entity {
            id: "crab_3".to_string(),
            x: 1.0, y: 5.0, z: 0.0,
            vx: 0.5, vy: -1.0,
            symbol: "🦀", color: "#ffe66d",
        },
    ];

    WorldState {
        entities,
        tick: 0,
        paused: true,
        speed: 1.0,
        width: 8,
        height: 6,
    }
}

struct PluginState {
    world: WorldState,
    running: bool,
    orchestrator_url: String,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            world: create_world(),
            running: false,
            orchestrator_url: std::env::var("ORCHESTRATOR_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string()),
        }
    }
}

/// Publish entity state to the orchestrator's SSE channel for crabjar-gpui subscribers.
async fn publish_entity_state(state: &PluginState) {
    let entities = state.world.entities.iter().map(|e| serde_json::json!({
        "id": e.id,
        "x": e.x,
        "y": e.y,
        "z": e.z,
        "symbol": e.symbol,
        "color": e.color
    })).collect::<Vec<_>>();

    let event = serde_json::json!({
        "type": "entity_state",
        "source": "terrarium",
        "tick": state.world.tick,
        "entities": entities
    });

    let payload = serde_json::json!({ "data": event });

    if let Ok(client) = reqwest::Client::builder().build() {
        let url = format!("{}/acp/events/publish", state.orchestrator_url);
        let _ = client.post(&url).json(&payload).send().await;
    }
}

// Simple physics step — move entities by velocity, bounce off walls
fn step_world(world: &mut WorldState) {
    world.tick += 1;
    for entity in world.entities.iter_mut() {
        let dx = entity.vx * 0.033 * world.speed;
        let dy = entity.vy * 0.033 * world.speed;

        entity.x += dx;
        entity.y += dy;

        // Bounce off walls
        if entity.x < 0.0 || entity.x > world.width as f32 {
            entity.vx = -entity.vx;
            entity.x = entity.x.clamp(0.0, world.width as f32);
        }
        if entity.y < 0.0 || entity.y > world.height as f32 {
            entity.vy = -entity.vy;
            entity.y = entity.y.clamp(0.0, world.height as f32);
        }
    }
}

// ============================================================================
// Command Handler
// ============================================================================

async fn handle_commands(state: &Mutex<PluginState>) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut output = stdout.lock();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => continue,
        }

        let request: CommandRequest = match serde_json::from_str(&line.trim()) {
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

        let result = {
            let mut state_guard = state.lock().await;
            handle_command(&mut state_guard, &request)
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

fn handle_command(state: &mut PluginState, request: &CommandRequest) -> serde_json::Value {
    match request.method.as_str() {
        "terrarium/start" => {
            state.running = true;
            state.world.paused = false;
            serde_json::json!({"status": "started"})
        }
        "terrarium/stop" => {
            state.running = false;
            state.world.paused = true;
            serde_json::json!({"status": "stopped"})
        }
        "terrarium/pause" => {
            state.world.paused = true;
            serde_json::json!({"status": "paused"})
        }
        "terrarium/resume" => {
            state.world.paused = false;
            serde_json::json!({"status": "resumed"})
        }
        "terrarium/set_speed" => {
            if let Some(params) = &request.params {
                if let Some(val) = &params.value {
                    state.world.speed = val.parse().unwrap_or(1.0);
                    serde_json::json!({"status": "speed_set", "speed": state.world.speed})
                } else {
                    serde_json::json!({"status": "error", "message": "Missing speed value"})
                }
            } else {
                serde_json::json!({"status": "error", "message": "Missing params"})
            }
        }
        "terrarium/step" => {
            if !state.world.paused {
                step_world(&mut state.world);
            }
            serde_json::json!({"status": "stepped", "tick": state.world.tick})
        }
        "terrarium/query_state" => {
            // Return ACTUAL entity positions from the world
            let entities = state.world.entities.iter().map(|e| serde_json::json!({
                "id": e.id,
                "x": e.x,
                "y": e.y,
                "z": e.z,
                "symbol": e.symbol,
                "color": e.color
            })).collect::<Vec<_>>();

            serde_json::json!({
                "status": "ok",
                "entities": entities,
                "tick": state.world.tick,
                "paused": state.world.paused,
                "speed": state.world.speed
            })
        }
        other => {
            serde_json::json!({"status": "error", "message": format!("Unknown method: {}", other)})
        }
    }
}

// ============================================================================
// Render Loop — updates entity positions over time
// ============================================================================

async fn render_loop(state: &Mutex<PluginState>) {
    loop {
        let should_run = {
            let state_guard = state.lock().await;
            state_guard.running
        };

        if !should_run {
            break;
        }

        let tick_result = {
            let mut state_guard = state.lock().await;
            if !state_guard.world.paused {
                step_world(&mut state_guard.world);
            }
            // Publish entity positions to orchestrator SSE channel
            publish_entity_state(&state_guard).await;
            state_guard.world.tick
        };

        sleep(Duration::from_millis(50)).await; // 20 FPS
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("stdio");

    match mode {
        "stdio" => {
            eprintln!("🦀 Terrarium plugin started (stdio mode)");

            let state = Mutex::new(PluginState::default());

            // Spawn command handler and render loop with shared state
            tokio::join!(handle_commands(&state), render_loop(&state));
        }
        "text" => {
            println!("🦀 Terrarium plugin started (text mode)");
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}
