use agent_context::state_docs::schema as state_schema;
use agent_context::{Source, Store, store::StoreError};
use axum::{
    Json, Router,
    extract::{Path, State},
    response::sse::{Event as SseEvent, Sse},
    routing::{get, post},
};
use std::env;


use crabjar_guard::{ActionStatus, ExecutionGate, GateConcierge, GateContext};

use futures_util::stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;

mod backend;
mod clarify;
mod lm_studio_client;
mod process_management;
mod session_search;
mod tools;

use backend::InferenceBackend;
use lm_studio_client::LmStudioClient;

use axum::middleware;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Represents the incoming request to run a command.
#[derive(Debug, Deserialize)]
struct RunRequest {
    tool: String,
    args: Vec<String>,
}

/// Represents the incoming request for a prompt.
#[derive(Debug, Deserialize)]
struct PromptRequest {
    message: String,
}

/// Represents an OpenAI-compatible Chat Completion request.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

/// Represents a single message in the chat history.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum MessageRole {
    System,
    User,
    Assistant,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: MessageRole,
    content: String,
}

/// Represents the incoming request for a chat interaction.
#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
    #[allow(dead_code)]
    model: Option<String>,
}

/// Represents an OpenAI-compatible Chat Completion response content.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    tool_calls: Option<Vec<ToolCall>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ToolCall {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    r#type: String,
    function: FunctionCall,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

/// Represents a search_logs request.
#[derive(Debug, Deserialize)]
struct SearchLogsRequest {
    term: String,
    limit: Option<i64>,
}

/// Represents a recent_events request.
#[derive(Debug, Deserialize)]
struct RecentEventsRequest {
    limit: i64,
}

/// Represents a by_source request.
#[derive(Debug, Deserialize)]
struct BySourceRequest {
    source: String,
    limit: Option<i64>,
}

/// Represents the outgoing messages from the Orchestrator to the Client via SSE or JSON.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
enum AcpResponse {
    /// Send real-time output/logs.
    Output { data: String },
    /// Ask the user for input.
    Input { message: String },
    /// Signal task finalization.
    Done { status: String },
    /// Report an error.
    Error { error: String },
}

/// Shared SSE event channel for live updates (e.g., terrarium entity positions).
type EventChannel = Arc<tokio::sync::Mutex<SseSubscriptionManager>>;

/// SSE subscription manager with inactive subscriber cleanup (#18 security finding).
struct SseSubscriptionManager {
    sender: tokio::sync::broadcast::Sender<String>,
}

impl SseSubscriptionManager {
    fn new(capacity: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity);
        Self { sender: tx }
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    fn send(
        &self,
        data: String,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<String>> {
        self.sender.send(data)
    }

    /// Recreate the channel to clear all subscribers (forces reconnection).
    /// Returns a new manager instance.
    fn reset(&self) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        info!("SSE subscription manager reset — cleared all subscribers");
        Self { sender: tx }
    }

    /// Get the current subscriber count.
    fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Spawn a background task to periodically clean up SSE subscribers (#18).
fn spawn_sse_cleanup(manager: Arc<tokio::sync::Mutex<SseSubscriptionManager>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let count = manager.lock().await.subscriber_count();
            if count > 50 {
                warn!(
                    count,
                    "SSE subscriber count exceeds threshold — resetting channel"
                );
                *manager.lock().await = manager.lock().await.reset();
            } else {
                info!(count, "SSE subscriber cleanup check");
            }
        }
    });
}

/// Subscribe to live events via SSE — used by crabjar-gpui for terrarium entity updates.
async fn subscribe_events(
    State(state): State<AppState>,
) -> Sse<impl stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = state.event_channel.lock().await.subscribe();

    let event_stream = stream::unfold(rx, |mut rx| async move {
        match tokio::time::timeout(tokio::time::Duration::from_secs(30), rx.recv()).await {
            Ok(Ok(event_data)) => Some((Ok(SseEvent::default().data(event_data)), rx)),
            Ok(Err(_)) => None, // Channel closed
            Err(_) => {
                // Timeout — send keepalive
                Some((Ok(SseEvent::default().data("{\"type\":\"keepalive\"}")), rx))
            }
        }
    });

    Sse::new(event_stream)
}

/// Publish an event to all SSE subscribers.
async fn publish_event(
    State(state): State<AppState>,
    Json(payload): Json<PublishRequest>,
) -> Json<AcpResponse> {
    let data = serde_json::to_string(&payload.data).unwrap_or_default();
    let manager = state.event_channel.lock().await;
    if manager.send(data).is_err() {
        return Json(AcpResponse::Error {
            error: "No subscribers".to_string(),
        });
    }
    Json(AcpResponse::Output {
        data: "Event published".to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Gate concierge — provenance boundary enforcement (guard's GateConcierge is the sole gate layer)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Events are now stored and queried via the agent-context Store.
// Mirror-lab paths are optional overrides via environment variables.
// ---------------------------------------------------------------------------

/// Handler for running a command and streaming its output via SSE.
async fn handle_run(
    Json(payload): Json<RunRequest>,
) -> Sse<impl stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let (tx, rx) = mpsc::channel::<AcpResponse>(100);
    let tool = payload.tool;
    let args = payload.args;

    // Spawn a task to manage the process execution and pipe output to the channel.
    tokio::spawn(async move {
        info!("Executing command: {} with args: {:?}", tool, args);

        let mut child = match Command::new(&tool)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = tx
                    .send(AcpResponse::Error {
                        error: e.to_string(),
                    })
                    .await;
                return;
            }
        };

        let stdout = child.stdout.take().expect("Failed to take stdout");
        let stderr = child.stderr.take().expect("Failed to take stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        // Monitor stdout, stderr, and the process exit status.
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            let _ = tx.send(AcpResponse::Output { data: l }).await;
                        }
                        Ok(None) => {}, // EOF for stdout
                        Err(e) => {
                            let _ = tx.send(AcpResponse::Error { error: e.to_string() }).await;
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            let _ = tx.send(AcpResponse::Output { data: l }).await;
                        }
                        Ok(None) => {}, // EOF for stderr
                        Err(e) => {
                            let _ = tx.send(AcpResponse::Error { error: e.to_string() }).await;
                            break;
                        }
                    }
                }
                status = child.wait() => {
                    match status {
                        Ok(exit_status) => {
                            let _ = tx.send(AcpResponse::Done {
                                status: format!("Exit code: {}", exit_status)
                            }).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AcpResponse::Error { error: e.to_string() }).await;
                        }
                    }
                    break;
                }
            }
        }
    });

    // Convert the mpsc receiver into an SSE stream.
    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(response) => {
                let data = serde_json::to_string(&response).unwrap_or_default();
                Some((Ok(SseEvent::default().data(data)), rx))
            }
            None => None, // Stream ends when the channel is closed.
        }
    });

    Sse::new(stream)
}

/// Handler for prompt requests (standard JSON response).
async fn handle_prompt(Json(payload): Json<PromptRequest>) -> Json<AcpResponse> {
    info!("Received prompt: {}", payload.message);
    Json(AcpResponse::Input {
        message: format!("Acknowledged prompt: {}", payload.message),
    })
}

/// Handler for chat requests — uses the configured inference backend.
#[axum::debug_handler]
async fn handle_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let user_input = payload.prompt;

    let mut backend = state.backend.lock().await;
    info!("Chat request received (backend: {})", (*backend).kind());

    let (response, metrics) = backend.generate(user_input).await.map_err(|e| {
        error!("Inference backend error: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(
        "Inference complete: model={}, tokens_in={}, tokens_out={}, latency={}ms",
        metrics.model_id, metrics.tokens_in, metrics.tokens_out, metrics.latency_ms
    );

    // Check for tool calls first.
    let tool_calls = backend.extract_tool_calls(&response);
    if !tool_calls.is_empty() {
        let mut results = Vec::new();
        for tc in tool_calls {
            info!(
                "LLM requested tool call: {} with args: {}",
                tc.tool, tc.arguments
            );

            let args: Vec<String> = match serde_json::from_str(&tc.arguments.to_string()) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error!("Failed to parse tool arguments: {}", e);
                    results.push(format!("Error parsing arguments for {}: {}", tc.tool, e));
                    continue;
                }
            };

            let tool_result = execute_tool_call(
                &tc.tool,
                &args,
                Arc::clone(&state.store),
                Arc::clone(&state.process_registry),
            )
            .await;
            results.push(format!(
                "Tool '{}' executed: {}",
                tc.tool,
                tool_result.unwrap_or_else(|e| e)
            ));
        }

        Ok(Json(AcpResponse::Output {
            data: results.join("\n"),
        }))
    } else {
        let content = backend.extract_text(&response);
        if content.is_empty() {
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            info!("LLM Response: {}", content);
            Ok(Json(AcpResponse::Output { data: content }))
        }
    }
}

/// Guard check + concierge enforcement — shared by all tool invocations.
/// Returns Ok(()) if approved, Err with reason if pending/denied/gate error.
fn guard_tool_call(tool_name: &str, args: &[String], source_event_id: &str) -> Result<(), String> {
    let guard_root = std::env::var("MIRROR_GUARD_ROOT").unwrap_or_else(|_| ".".to_string());

    let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(format!(
        "{}/guard.db",
        guard_root,
    )))
    .unwrap_or_else(|_| {
        warn!(
            "Failed to open guard DB for '{}', using in-memory fallback",
            tool_name
        );
        crabjar_guard::GuardDb::open(":memory:").unwrap()
    });

    let gate = ExecutionGate::new(&guard_db, false, &guard_root);
    let actor_scope = crabjar_guard::Scope::project("orchestrator");
    let target_scope = actor_scope.clone();
    let mut concierge = GateConcierge::new().with_db(guard_db.clone());

    match gate.check(GateContext {
        action_type: "tool_call",
        command: tool_name,
        args: args.to_vec(),
        trust_layer: 2,
        confidence: crabjar_guard::TrustScore::new(0.5),
        source_event_id: Some(source_event_id),
        can_interrupt: true,
        pid: None,
        scope: Some(actor_scope.clone()),
        target_scope: Some(target_scope.clone()),
        cross_scope_auth: None,
        domains: vec![],
        context_budget: None,
        context_fragment_tokens: None,
    }) {
        Ok(result) => {
            let outcome = concierge.enforce(
                result,
                "tool_call",
                tool_name,
                args,
                2,
                0.5,
                Some(source_event_id.to_string()),
            );

            match outcome {
                crabjar_guard::ConciergeOutcome::Approved(_) => {
                    info!("Gate concierge: Proceed — {}", tool_name);
                    Ok(())
                }
                crabjar_guard::ConciergeOutcome::Queued(_, pending_entry) => Err(format!(
                    "Pending: queued for review (pending_id: {})",
                    pending_entry.id
                )),
                crabjar_guard::ConciergeOutcome::Denied(_, interrupted_entry) => Err(format!(
                    "Interrupted: {} (interrupted_id: {})",
                    interrupted_entry.reason,
                    interrupted_entry.id
                )),
            }
        }
        Err(e) => {
            error!("Security gate error for '{}': {}", tool_name, e);
            Err(format!("Security gate error: {}", e))
        }
    }
}

/// Execute a binary with guard gate enforcement, capturing stdout/stderr.
async fn execute_with_guard(
    tool_name: &str,
    _args: &[String],
    binary_path: &str,
    _project_root: &std::path::Path,
    _store: &std::sync::Mutex<Store>,
) -> Result<String, String> {
    let source_event_id = format!("orchestrator-tr-{}", tool_name);

    // Guard check (shared helper)
    guard_tool_call(tool_name, _args, &source_event_id)?;

    // Execute the binary
    let mut child = match tokio::process::Command::new(binary_path)
        .args(_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return Err(format!("Error spawning '{}': {}", tool_name, e));
        }
    };

    let stdout = child.stdout.take().expect("Failed to take stdout");
    let stderr = child.stderr.take().expect("Failed to take stderr");

    let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

    let mut output = String::new();

    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        output.push_str(&l);
                        output.push('\n');
                    }
                    Ok(None) => {}
                    Err(e) => {
                        output.push_str(&format!("Error reading stdout: {}", e));
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        output.push_str(&format!("stderr: {}\n", l));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        output.push_str(&format!("Error reading stderr: {}", e));
                        break;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        output.push_str(&format!("\nExit code: {}", exit_status));
                    }
                    Err(e) => {
                        output.push_str(&format!("\nError waiting for process: {}", e));
                    }
                }
                break;
            }
        }
    }

    Ok(output)
}

/// Validate tool name — prevent path traversal and injection via tool names.
fn validate_tool_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    // Allow lowercase alphanumeric, hyphens, underscores (standard tool naming)
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Resolve and execute a tool call via the tool registry, falling back to
/// the built-in tool dispatch for backward compatibility.
async fn execute_tool_call(
    function_name: &str,
    args: &[String],
    store: Arc<std::sync::Mutex<Store>>,
    process_registry: Arc<Mutex<process_management::ProcessRegistry>>,
) -> Result<String, String> {
    // Validate tool name to prevent path traversal and injection (#16 security finding)
    if !validate_tool_name(function_name) {
        warn!(tool = %function_name, "Invalid tool name rejected");
        return Err(format!("Invalid tool name: {}", function_name));
    }
    // --- Attempt tool registry resolution first ---
    let project_root = std::env::var("CRABJAR_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .unwrap_or_default();
    let tool_registry_path = project_root.join("tool_registry/tool_registry.db");

    // All sync work before any await (Connection is not Send)
    let discovered_tools = if tool_registry_path.exists() {
        match rusqlite::Connection::open(&tool_registry_path) {
            Ok(conn) => {
                let registry = crabjar_tool_registry::ToolRegistry::new(&conn);
                if registry.init().is_ok() {
                    registry
                        .discover_tools("orchestrator", &project_root)
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let mut validation: Option<Vec<(String, bool, Option<String>)>> = None;

    if !discovered_tools.is_empty() {
        validation = Some(
            crabjar_tool_registry::ToolRegistry::new(
                &rusqlite::Connection::open(&tool_registry_path)
                    .unwrap_or_else(|_| rusqlite::Connection::open(":memory:").unwrap()),
            )
            .validate_tools(&discovered_tools)
            .unwrap_or_default(),
        );
    }

    // Check if the requested tool is registered and available
    let tool_available = if let Some(ref v) = validation {
        v.iter()
            .any(|(name, avail, _)| name == function_name && *avail)
    } else {
        false
    };

    // If a registered tool exists and is available, use it; otherwise fall through to built-in dispatch
    if tool_available {
        // Validate: check if binary is actually present
        let binary_path = validation
            .as_ref()
            .and_then(|v| v.iter().find(|(n, _, _)| n == function_name))
            .and_then(|(_, _, p)| p.clone());

        if let Some(ref path) = binary_path {
            // Execute via the guard gate (same path as built-in tools)
            return execute_with_guard(function_name, args, path, &project_root, store.as_ref())
                .await;
        } else {
            // Tool registered but binary missing — return a helpful error
            return Err(format!(
                "Tool '{}' is registered but binary not found in PATH. Install it or run 'crabjar tool discover' to update the registry.",
                function_name
            ));
        }
    }

    // --- Fallback: built-in tool dispatch (backward compatibility) ---
    let ctx = tools::ToolContext {
        store: store.clone(),
        process_registry,
    };
    tools::execute(function_name, args, &ctx).await
}

/// Format search results into a readable string for the user.
fn format_results(response: &crabjar_web_search::SearchResponse) -> Result<String, String> {
    if response.results.is_empty() {
        return Ok("No search results found.".to_string());
    }

    let mut output = String::new();
    output.push_str(&format!(
        "Found {} search results:\n\n",
        response.results.len()
    ));

    for (i, result) in response.results.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, result.title));
        output.push_str(&format!("   URL: {}\n", result.url));
        if let Some(desc) = &result.description
            && !desc.is_empty()
        {
            output.push_str(&format!("   Description: {}\n", desc));
        }
        if let Some(snippet) = &result.snippet
            && !snippet.is_empty()
        {
            output.push_str(&format!("   Snippet: {}\n", snippet));
        }
        output.push('\n');
    }

    Ok(output)
}

/// Shared state for Axum handlers.
#[allow(dead_code)]
#[derive(Clone)]
struct AppState {
    store: Arc<std::sync::Mutex<Store>>,
    events_db_path: String,
    guard_root: String,
    backend: Arc<Mutex<Box<dyn InferenceBackend>>>,
    /// Scope for this orchestrator instance (used for gate context).
    actor_scope: crabjar_guard::Scope,
    target_scope: crabjar_guard::Scope,
    event_channel: EventChannel,
    process_registry: Arc<Mutex<process_management::ProcessRegistry>>,
}

/// Handler for recent_events — queries the knowledge store.
async fn recent_events(
    State(state): State<AppState>,
    Json(payload): Json<RecentEventsRequest>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let events = match state.store.lock().unwrap().events(payload.limit as usize) {
        Ok(events) => events,
        Err(StoreError::DatabaseError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::JsonError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::Internal(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut output = String::new();
    output.push_str(&format!("Recent {} events:\n", events.len()));

    for event in events.iter() {
        output.push_str(&format!(
            "[{}] {} - {}\n",
            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
            event.event_type,
            event.id,
        ));
    }

    Ok(Json(AcpResponse::Output { data: output }))
}

/// Handler for by_source — queries the knowledge store.
async fn by_source(
    State(state): State<AppState>,
    Json(payload): Json<BySourceRequest>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let events = match state.store.lock().unwrap().query(
        &[&payload.source],
        payload.limit.unwrap_or(50) as usize,
        "",
        "",
        "",
    ) {
        Ok(events) => events,
        Err(StoreError::DatabaseError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::JsonError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::Internal(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut output = String::new();
    output.push_str(&format!("Events from source '{}':\n", payload.source));

    for event in events.iter() {
        output.push_str(&format!(
            "[id:{}] {}\n",
            event.id,
            &event.content[..event.content.len().min(200)]
        ));
    }

    Ok(Json(AcpResponse::Output { data: output }))
}

/// Handler for search_logs — queries the knowledge store.
async fn search_logs(
    State(state): State<AppState>,
    Json(payload): Json<SearchLogsRequest>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let events = match state
        .store
        .lock()
        .unwrap()
        .search_content(&payload.term, payload.limit.map(|l| l as usize))
    {
        Ok(events) => events,
        Err(StoreError::DatabaseError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::JsonError(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(StoreError::Internal(_e)) => {
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut output = String::new();
    output.push_str(&format!(
        "Found {} entries matching '{}':\n",
        events.len(),
        payload.term
    ));

    for event in events.iter().take(payload.limit.unwrap_or(10) as usize) {
        output.push_str(&format!(
            "[id:{}] {}\n",
            event.id,
            &event.content[..event.content.len().min(200)]
        ));
    }

    Ok(Json(AcpResponse::Output { data: output }))
}

// ---------------------------------------------------------------------------
// P1-5: Interactive Clarify Tooling — API endpoints for TUI integration
// ---------------------------------------------------------------------------

/// Request body for submitting a clarify response.
#[derive(Debug, Deserialize)]
struct ClarifyResponseRequest {
    /// The selected option value (for choice-based questions) or null.
    #[serde(default)]
    value: Option<String>,
    /// Free-text response (for open-ended questions).
    #[serde(default)]
    text: Option<String>,
}

/// Submit a response to a pending clarify question.
async fn clarify_response(
    State(state): State<AppState>,
    Path(question_id): Path<String>,
    Json(payload): Json<ClarifyResponseRequest>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    info!(
        "Received clarify response for question {}: value={:?}, text={:?}",
        question_id, payload.value, payload.text
    );

    let store = state.store.lock().unwrap();

    // Find the pending question by its specific tag
    let entries = match store.query(&[&format!("clarify-{}", question_id)], 1, "", "", "") {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to query clarify question {}: {}", question_id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if entries.is_empty() {
        return Err(axum::http::StatusCode::NOT_FOUND);
    }

    let row = &entries[0];

    // Check if already answered
    if let Some(status) = row.metadata.get("status").and_then(|v| v.as_str())
        && status == "answered"
    {
        return Err(axum::http::StatusCode::CONFLICT);
    }

    // Update the question entry with the response via raw SQL
    let updated_metadata = serde_json::json!({
        "question_id": question_id,
        "status": "answered",
        "response_value": payload.value.clone(),
        "response_text": payload.text.clone(),
        "created_at": row.metadata.get("created_at"),
        "answered_at": chrono::Utc::now().to_rfc3339()
    });

    let metadata_str = match serde_json::to_string(&updated_metadata) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to serialize updated metadata: {}", e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let tags_str =
        serde_json::to_string(&vec![format!("clarify-{}", question_id)]).unwrap_or_default();

    match store.conn.execute(
        "UPDATE knowledge_entries SET metadata = ?, tags = ? WHERE id = ?",
        rusqlite::params![metadata_str, tags_str, row.id],
    ) {
        Ok(_) => {
            info!("Clarify question {} marked as answered", question_id);
            Ok(Json(AcpResponse::Output {
                data: format!("Response recorded for question {}", question_id),
            }))
        }
        Err(e) => {
            error!("Failed to update clarify question {}: {}", question_id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Check the status of a clarify question and retrieve the response if available.
async fn clarify_status(
    State(state): State<AppState>,
    Path(question_id): Path<String>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let store = state.store.lock().unwrap();

    // Find the question by its specific tag
    let entries = match store.query(&[&format!("clarify-{}", question_id)], 1, "", "", "") {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to query clarify question {}: {}", question_id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if entries.is_empty() {
        return Err(axum::http::StatusCode::NOT_FOUND);
    }

    let row = &entries[0];

    // Determine status from metadata
    let status = row
        .metadata
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("pending");

    match status {
        "answered" => {
            let response_value = row.metadata.get("response_value").and_then(|v| v.as_str());
            let response_text = row.metadata.get("response_text").and_then(|v| v.as_str());

            Ok(Json(AcpResponse::Output {
                data: serde_json::json!({
                    "status": "answered",
                    "question_id": question_id,
                    "value": response_value,
                    "text": response_text,
                    "answered_at": row.metadata.get("answered_at").and_then(|v| v.as_str())
                })
                .to_string(),
            }))
        }
        _ => {
            // Check for timeout
            let created_at = row.metadata.get("created_at").and_then(|v| v.as_str());

            if let Some(createdAt) = created_at
                && let Ok(dt) = chrono::DateTime::parse_from_rfc3339(createdAt)
            {
                // Convert to UTC for comparison
                let dt_utc = dt.with_timezone(&chrono::Utc);
                let elapsed = chrono::Utc::now() - dt_utc;
                // Default timeout is 300 seconds (5 minutes)
                if elapsed.num_seconds() > 300 {
                    return Ok(Json(AcpResponse::Output {
                        data: serde_json::json!({
                            "status": "timed_out",
                            "question_id": question_id,
                            "elapsed_seconds": elapsed.num_seconds()
                        })
                        .to_string(),
                    }));
                }
            }

            Ok(Json(AcpResponse::Output {
                data: serde_json::json!({
                    "status": "pending",
                    "question_id": question_id
                })
                .to_string(),
            }))
        }
    }
}

/// List all pending (unanswered) clarify questions.
async fn clarify_pending(
    State(state): State<AppState>,
) -> Result<Json<AcpResponse>, axum::http::StatusCode> {
    let store = state.store.lock().unwrap();

    // Clean up old answered/expired questions older than 24 hours (#13 security finding: memory leak)
    // Deactivate them so they don't accumulate forever in the database.
    let now = chrono::Utc::now().timestamp_millis();
    let twenty_four_hours_ms = 86_400_000;
    if let Ok(all_entries) = store.query(&["clarify-pending"], 500, "", "", "") {
        for entry in &all_entries {
            if let Some(created_at_str) = entry.metadata.get("created_at").and_then(|v| v.as_str())
                && let Ok(created_at) = created_at_str.parse::<i64>()
                && now - created_at > twenty_four_hours_ms
            {
                // Entry is older than 24 hours — deactivate it
                let _ = store.deactivate(entry.id, Source::System, Some("expired: older than 24h"));
            }
        }
    }

    // Query for all entries tagged with "clarify-pending"
    let entries = match store.query(&["clarify-pending"], 100, "", "", "") {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to query pending clarify questions: {}", e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let questions: Vec<serde_json::Value> = entries
        .iter()
        .filter_map(|entry| {
            // Parse the content back to ClarifyInput
            match serde_json::from_str::<clarify::ClarifyInput>(&entry.content) {
                Ok(input) => Some(serde_json::json!({
                    "question_id": entry.metadata.get("question_id").and_then(|v| v.as_str()),
                    "question": input.question,
                    "options": input.options,
                    "timeout_secs": input.timeout_secs,
                    "created_at": entry.metadata.get("created_at").and_then(|v| v.as_str())
                })),
                Err(_) => None,
            }
        })
        .collect();

    Ok(Json(AcpResponse::Output {
        data: serde_json::json!({
            "pending_count": questions.len(),
            "questions": questions
        })
        .to_string(),
    }))
}

// ---------------------------------------------------------------------------
// API Key Authentication Middleware (#3 security finding)
// ---------------------------------------------------------------------------

/// Validate API key from Authorization header or api_key query parameter.
/// If ORCHESTRATOR_API_KEY env var is not set, all requests are allowed (backward compatible).
async fn api_key_auth(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let expected_key = std::env::var("ORCHESTRATOR_API_KEY");

    // If no API key is configured, allow all requests (backward compatibility)
    if expected_key.is_err() || expected_key.as_ref().unwrap().is_empty() {
        return Ok(next.run(req).await);
    }

    let expected = expected_key.unwrap();

    // Check Authorization: Bearer *** header first
    if let Some(auth_header) = req.headers().get("authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        // Constant-time comparison to prevent timing attacks
        if subtle::ConstantTimeEq::ct_eq(token.as_bytes(), expected.as_bytes()).into() {
            return Ok(next.run(req).await);
        }
    }

    // Also check api_key query parameter for simpler clients
    let uri = req.uri();
    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next())
                && key == "api_key"
                && subtle::ConstantTimeEq::ct_eq(value.as_bytes(), expected.as_bytes()).into()
            {
                return Ok(next.run(req).await);
            }
        }
    }

    warn!("API key authentication failed");
    Err(axum::http::StatusCode::UNAUTHORIZED)
}

// ---------------------------------------------------------------------------
// Rate Limiting Middleware (#10 security finding)
// ---------------------------------------------------------------------------

/// Persistent rate limiter: tracks requests per IP, limits to N per minute.
/// Uses file-based persistence across conductor restarts (redteam finding #15).
struct RateLimiter {
    state_file: std::path::PathBuf,
    max_requests: u32,
    window_seconds: i64,
    // In-memory cache of recent requests
    requests: HashMap<String, Vec<i64>>,
}

impl RateLimiter {
    fn new(max_requests: u32, window_seconds: i64) -> Self {
        let state_dir = env::var("CRABJAR_STATE_DIR")
            .expect("CRABJAR_STATE_DIR environment variable must be set");
        let state_file = std::path::PathBuf::from(state_dir).join("crabjar_rate_limits.json");
        let mut limiter = Self {
            state_file,
            max_requests,
            window_seconds,
            requests: HashMap::new(),
        };
        limiter.load_state();
        limiter
    }

    fn load_state(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.state_file)
            && let Ok(state) = serde_json::from_str::<HashMap<String, Vec<i64>>>(&content)
        {
            // Filter out stale entries
            let now = chrono::Utc::now().timestamp();
            for (ip, timestamps) in state {
                let recent: Vec<i64> = timestamps
                    .into_iter()
                    .filter(|&t| now - t < self.window_seconds)
                    .collect();
                if !recent.is_empty() {
                    self.requests.insert(ip, recent);
                }
            }
        }
    }

    fn save_state(&self) {
        let _ = std::fs::write(
            &self.state_file,
            serde_json::to_string(&self.requests).unwrap_or_default(),
        );
    }

    fn allow(&mut self, ip: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let window_start = now - self.window_seconds;

        // Clean up old entries for this IP
        if let Some(timestamps) = self.requests.get_mut(ip) {
            timestamps.retain(|&t| t > window_start);
        } else {
            self.requests.insert(ip.to_string(), Vec::new());
        }

        let timestamps = self.requests.get_mut(ip).unwrap();

        // Check if within limit
        if timestamps.len() >= self.max_requests as usize {
            return false;
        }

        // Add current timestamp
        timestamps.push(now);
        self.save_state();
        true
    }
}

/// Rate limiting middleware — extracts client IP and enforces rate limits.
async fn rate_limit_middleware(
    state: axum::extract::State<Arc<Mutex<RateLimiter>>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    // Extract client IP from headers or use default
    let ip = if let Some(x_forwarded_for) = req.headers().get("x-forwarded-for") {
        x_forwarded_for
            .to_str()
            .unwrap_or("unknown")
            .split(",")
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string()
    } else {
        "unknown".to_string()
    };

    let mut limiter = state.0.lock().await;
    if !limiter.allow(&ip) {
        warn!(ip = %ip, "Rate limit exceeded");
        return Err(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// TLS Configuration (#12 security finding)
// ---------------------------------------------------------------------------

/// Load TLS configuration from cert/key files if ORCHESTRATOR_TLS_CERT and
/// ORCHESTRATOR_TLS_KEY environment variables are set. Returns None for HTTP fallback.
async fn load_tls_config() -> Option<axum_server::tls_rustls::RustlsConfig> {
    let cert_path = match std::env::var("ORCHESTRATOR_TLS_CERT") {
        Ok(path) => path,
        Err(_) => {
            info!("TLS not configured (ORCHESTRATOR_TLS_CERT not set) — using HTTP");
            return None;
        }
    };

    let key_path = match std::env::var("ORCHESTRATOR_TLS_KEY") {
        Ok(path) => path,
        Err(_) => {
            warn!("ORCHESTRATOR_TLS_CERT set but ORCHESTRATOR_TLS_KEY not set — using HTTP");
            return None;
        }
    };

    info!(
        "Loading TLS configuration from {} / {}",
        cert_path, key_path
    );

    match axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await {
        Ok(config) => {
            info!("TLS configuration loaded successfully");
            Some(config)
        }
        Err(e) => {
            error!("Failed to load TLS configuration: {}", e);
            None
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for structured logging.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    // Database configuration — mirror-lab paths are optional overrides
    let crabjar_root = std::env::var("CRABJAR_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let knowledge_db_path = std::env::var("KNOWLEDGE_DB_PATH")
        .unwrap_or_else(|_| format!("{}/memory/knowledge.db", crabjar_root.display()));
    let events_db_path = std::env::var("MIRROR_LOG_DB_PATH")
        .unwrap_or_else(|_| format!("{}/memory/events.db", crabjar_root.display()));
    let guard_root = std::env::var("MIRROR_GUARD_ROOT")
        .unwrap_or_else(|_| std::env::var("CRABJAR_ROOT").unwrap_or_else(|_| ".".to_string()));

    // Initialize knowledge store schema
    let kconn = rusqlite::Connection::open(&knowledge_db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open knowledge DB: {e}"))?;
    state_schema::migrate(&kconn).map_err(|e| anyhow::anyhow!("Schema migration failed: {e}"))?;
    let store = Store::open(&knowledge_db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open knowledge store: {e}"))?;

    // Initialize default inference backend (LM Studio client).
    // Native/PESTI inference was moved to the PESTI portable execution substrate.
    let backend: Box<dyn InferenceBackend> = Box::new(LmStudioClient::from_env());

    // Shared state across request handlers — construct scope for gate context
    let actor_scope = crabjar_guard::Scope::project("orchestrator");
    let target_scope = actor_scope.clone();

    // SSE event broadcast channel (capacity 100, oldest dropped if full)
    let event_channel: EventChannel =
        Arc::new(tokio::sync::Mutex::new(SseSubscriptionManager::new(100)));

    // Shared state across request handlers
    let state = AppState {
        store: Arc::new(std::sync::Mutex::new(store)),
        events_db_path,
        guard_root,
        backend: Arc::new(Mutex::new(backend)),
        actor_scope,
        target_scope,
        event_channel: event_channel.clone(),
        process_registry: Arc::new(Mutex::new(process_management::ProcessRegistry::new())),
    };

    // Spawn SSE subscriber cleanup task (#18 security finding)
    spawn_sse_cleanup(event_channel);

    // Define the Axum router with SSE and JSON endpoints.
    let app = Router::new()
        .route("/acp/run", post(handle_run))
        .route("/acp/prompt", post(handle_prompt))
        .route("/acp/chat", post(handle_chat))
        .route("/acp/search_logs", post(search_logs))
        .route("/acp/recent_events", post(recent_events))
        .route("/acp/by_source", post(by_source))
        .route(
            "/acp/events/subscribe",
            axum::routing::get(subscribe_events),
        )
        .route("/acp/events/publish", post(publish_event))
        // P1-5: Interactive Clarify Tooling — API endpoints for TUI integration
        .route("/acp/clarify/response/:id", post(clarify_response))
        .route("/acp/clarify/status/:id", get(clarify_status))
        .route("/acp/clarify/pending", get(clarify_pending))
        .layer(middleware::from_fn(api_key_auth))
        .with_state(state);

    // Restrict CORS to localhost origins only (#4 security finding)
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::exact(
            axum::http::HeaderValue::from_static("http://localhost:3000"),
        ))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            axum::http::HeaderName::from_static("content-type"),
            axum::http::HeaderName::from_static("authorization"),
        ]);

    // Rate limiter: 100 requests per minute per IP (#10 security finding)
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(100, 60)));

    let app = app.layer(cors).layer(middleware::from_fn_with_state(
        rate_limiter.clone(),
        rate_limit_middleware,
    ));

    // Bind to localhost:3000.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Load TLS configuration if configured (#12 security finding)
    let tls_acceptor = load_tls_config().await;

    if let Some(tls_config) = tls_acceptor {
        info!(
            "ACP Orchestrator listening on https://{} (TLS enabled)",
            addr
        );
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        info!("ACP Orchestrator listening on http://{} (no TLS)", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}
