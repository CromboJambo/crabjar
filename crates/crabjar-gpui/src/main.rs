//! Crabjar GPU Dashboard — connects to a real terrarium orchestrator via
//! SSE event subscription and JSON-RPC over stdio, renders live entity state
//! in a gpui dashboard.
//!
//! Architecture:
//! - Orchestrator spawns terrarium plugin (stdio JSON-RPC)
//! - Terrarium plugin publishes entity positions each tick to /acp/events/publish
//! - crabjar-gpui subscribes via SSE at /acp/events/subscribe
//! - Dashboard updates on live entity_state events

use futures_util::stream::StreamExt;
use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, IntoElement, ParentElement,
    Render, Window, WindowBounds, WindowOptions, FontWeight,
};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

// ============================================================================
// JSON-RPC Protocol (matches apps/terrarium/src/plugin.rs)
// ============================================================================

#[derive(Debug, Clone)]
struct CommandRequest {
    id: u64,
    method: String,
    params: Option<Value>,
}

impl CommandRequest {
    fn new(id: u64, method: &str, params: Option<Value>) -> Self {
        Self { id, method: method.to_string(), params }
    }

    fn to_json(&self) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": self.id,
            "method": self.method,
            "params": self.params
        })
        .to_string()
    }
}

#[derive(Debug, Clone)]
struct CommandResponse {
    id: u64,
    result: Option<Value>,
    error: Option<String>,
}

impl CommandResponse {
    fn parse(json: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(json).ok()?;
        Some(Self {
            id: v.get("id").and_then(|x| x.as_u64()).unwrap_or(0),
            result: v.get("result").cloned(),
            error: v.get("error").and_then(|x| x.as_str()).map(|s| s.to_string()),
        })
    }
}

// ============================================================================
// Entity Model (from DAGR events)
// ============================================================================

#[derive(Debug, Clone)]
struct EntityInfo {
    id: String,
    x: f32,
    y: f32,
    last_action: String,
}

// ============================================================================
// Orchestrator Connection (spawns terrarium plugin, JSON-RPC over stdio)
// ============================================================================

struct OrchestratorConnection {
    stdin: ChildStdin,
    stdout_reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl OrchestratorConnection {
    /// Spawn the terrarium plugin in JSON-RPC mode.
    fn spawn() -> anyhow::Result<Self> {
        let mut child = Command::new("cargo")
            .args(["run", "-p", "crabjar-app-terrarium", "--bin", "crabjar-terrarium-plugin", "--", "stdio"])
            .current_dir("/home/crombo/projects/active/crabjar")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // DAGR events go to terminal
            .spawn()?;

        let stdin = child.stdin.take().ok_or(anyhow::anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or(anyhow::anyhow!("No stdout"))?;

        Ok(Self {
            stdin,
            stdout_reader: BufReader::new(stdout),
            next_id: 1,
        })
    }

    /// Send a JSON-RPC request and wait for response.
    fn request(&mut self, method: &str, params: Option<Value>) -> anyhow::Result<Value> {
        let msg = CommandRequest::new(self.next_id, method, params);
        self.next_id += 1;

        self.stdin.write_all(msg.to_json().as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;

        let mut line = String::new();
        self.stdout_reader.read_line(&mut line)?;

        let response = CommandResponse::parse(line.trim())
            .ok_or(anyhow::anyhow!("Invalid JSON-RPC response: {}", line.trim()))?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("JSON-RPC error: {}", error));
        }

        Ok(response.result.unwrap_or(json!({})))
    }

    /// Query all entities from the orchestrator.
    fn get_entities(&mut self) -> anyhow::Result<Vec<EntityInfo>> {
        let result = self.request("terrarium/query_state", None)?;

        let mut entities = Vec::new();

        if let Some(array) = result.get("entities").and_then(|v| v.as_array()) {
            for item in array {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let x = item.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = item.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                entities.push(EntityInfo {
                    id,
                    x,
                    y,
                    last_action: "idle".to_string(),
                });
            }
        }

        Ok(entities)
    }
}

// ============================================================================
// SSE Event Subscription (live entity updates from orchestrator)
// ============================================================================

async fn subscribe_to_entity_events(
    orchestrator_url: &str,
    tx: tokio::sync::mpsc::Sender<Vec<EntityInfo>>,
) {
    let url = format!("{}/acp/events/subscribe", orchestrator_url);
    
    match reqwest::get(&url).await {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("SSE subscription failed: {}", response.status());
                return;
            }

            let mut stream = response.bytes_stream();
            
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        
                        // Parse SSE data lines
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let json_str = &line[6..];
                                
                                // Skip keepalive
                                if json_str == "{\"type\":\"keepalive\"}" {
                                    continue;
                                }
                                
                                if let Ok(event) = serde_json::from_str::<Value>(json_str) {
                                    if event.get("type").and_then(|v| v.as_str()) == Some("entity_state") {
                                        // Send parsed entities to the channel
                                        let entities = parse_entity_event(&event);
                                        if tx.send(entities).await.is_err() {
                                            return; // Channel closed
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("SSE stream error: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to SSE endpoint: {}", e);
        }
    }
}

fn parse_entity_event(event: &Value) -> Vec<EntityInfo> {
    let mut entities = Vec::new();
    
    if let Some(array) = event.get("entities").and_then(|v| v.as_array()) {
        for item in array {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let x = item.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = item.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            entities.push(EntityInfo {
                id,
                x,
                y,
                last_action: "idle".to_string(),
            });
        }
    }
    
    entities
}

struct DashboardView {
    entities: Vec<EntityInfo>,
    status_message: String,
}

impl DashboardView {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            status_message: "Connecting to terrarium...".to_string(),
        }
    }

    fn update_entities(&mut self, entities: Vec<EntityInfo>) {
        self.entities = entities;
        self.status_message = format!("{} entities", self.entities.len());
    }

    fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
    }
}

impl Render for DashboardView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(gpui::black())
            .text_color(gpui::white())
            .child(
                div().flex().items_center().gap_2().child("🦀 crabjar-gpui").child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x999999))
                        .child(format!("({})", self.status_message)),
                ),
            )
            .child(div().border_1().border_color(rgb(0x4d4d4d)).w_full())
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_lg()
                    .child("Entities"),
            )
            .children(self.entities.iter().map(|entity| {
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_2()
                    .bg(rgb(0x1a1a1a))
                    .border_1()
                    .border_color(rgb(0x61afef))
                    .child(entity.id.clone())
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x999999))
                            .child(format!("({:.1}, {:.1})", entity.x, entity.y)),
                    )
            }))
    }
}

// ============================================================================
// Main Loop
// ============================================================================

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| DashboardView::new()),
            )
            .unwrap();

        // Spawn orchestrator connection and update loop using cx.spawn (not std thread)
        let window_handle = window;
        let any_window: gpui::AnyWindowHandle = window_handle.into();
        
        // Start terrarium plugin via JSON-RPC
        let spawn_task = cx.spawn(async move |cx| {
            match OrchestratorConnection::spawn() {
                Ok(mut conn) => {
                    eprintln!("Terrarium plugin spawned successfully");
                    
                    // Send start command to begin simulation
                    if let Err(e) = conn.request("terrarium/start", None) {
                        eprintln!("Failed to start terrarium: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to spawn terrarium: {}", e);
                }
            }
        }).detach();

        // Subscribe to live entity events via SSE
        let orchestrator_url = std::env::var("ORCHESTRATOR_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        
        // Create channel for SSE events to communicate with the UI thread
        let (entity_tx, mut entity_rx) = tokio::sync::mpsc::channel::<Vec<EntityInfo>>(100);
        
        cx.spawn(async move |cx| {
            eprintln!("Subscribing to entity events at {}", orchestrator_url);
            subscribe_to_entity_events(&orchestrator_url, entity_tx).await;
        }).detach();

        // Listen for entity updates on the UI thread
        let window_handle = window;
        cx.spawn(async move |cx| {
            while let Some(entities) = entity_rx.recv().await {
                window_handle.update(cx, move |dash: &mut DashboardView, _, _| {
                    dash.set_status("Live");
                    dash.update_entities(entities);
                }).ok();
            }
        }).detach();

        cx.activate(true);
    });
}