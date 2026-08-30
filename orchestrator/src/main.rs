use agent_context::state_docs::schema as state_schema;
use agent_context::{Store, store::StoreError};
use axum::{
    Router,
    extract::Json,
    extract::State,
    response::sse::{Event as SseEvent, Sse},
    routing::post,
};
use crabjar_guard::{ActionStatus, ExecutionGate, GateConcierge, GateContext};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

mod backend;
mod lm_studio_client;

use backend::InferenceBackend;
use lm_studio_client::LmStudioClient;

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

    let response = backend.chat(user_input).await.map_err(|e| {
        error!("Inference backend error: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

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

            let tool_result = execute_tool_call(&tc.tool, &args, Arc::clone(&state.store)).await;
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

/// Execute a binary with guard gate enforcement, capturing stdout/stderr.
async fn execute_with_guard(
    tool_name: &str,
    _args: &[String],
    binary_path: &str,
    project_root: &std::path::Path,
    _store: &std::sync::Mutex<Store>,
) -> Result<String, String> {
    let guard_root = std::env::var("MIRROR_GUARD_ROOT")
        .unwrap_or_else(|_| project_root.to_string_lossy().to_string());

    let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(format!(
        "{}/guard.db",
        guard_root,
    )))
    .unwrap_or_else(|_| {
        warn!("Failed to open guard DB for tool registry execution, using in-memory fallback");
        crabjar_guard::GuardDb::open(":memory:").unwrap()
    });

    let gate = ExecutionGate::new(&guard_db, false, &guard_root);

    // Construct scope for this orchestrator instance
    let actor_scope = crabjar_guard::Scope::project("orchestrator");
    let target_scope = actor_scope.clone();
    let cross_scope_auth =
        crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);

    let mut concierge = GateConcierge::new().with_db(guard_db.clone());

    match gate.check(GateContext {
        action_type: "tool_call",
        command: tool_name,
        args: _args.to_vec(),
        trust_layer: 2,
        confidence: crabjar_guard::TrustScore::new(0.5),
        source_event_id: Some(&format!("orchestrator-tr-{}", tool_name)),
        can_interrupt: true,
        pid: None,
        scope: Some(actor_scope.clone()),
        target_scope: Some(target_scope.clone()),
        cross_scope_auth,
        domains: vec![],
        context_budget: None,
        context_fragment_tokens: None,
    }) {
        Ok(result) => {
            let (status, pending_entry, interrupted_entry) = concierge.enforce(
                result,
                "tool_call",
                tool_name,
                _args,
                2,
                0.5,
                Some(format!("orchestrator-tr-{}", tool_name)),
            );

            match status {
                ActionStatus::TrustApproved => {
                    info!("Gate concierge: Proceed — {} via tool registry", tool_name);
                }
                ActionStatus::Pending => {
                    return Err(format!(
                        "Pending: queued for review (pending_id: {})",
                        pending_entry
                            .as_ref()
                            .map(|e| e.id.clone())
                            .unwrap_or_default()
                    ));
                }
                ActionStatus::Denied => {
                    return Err(format!(
                        "Interrupted: {} (interrupted_id: {})",
                        interrupted_entry
                            .as_ref()
                            .map(|e| e.reason.clone())
                            .unwrap_or_default(),
                        interrupted_entry
                            .as_ref()
                            .map(|e| e.id.clone())
                            .unwrap_or_default()
                    ));
                }
                ActionStatus::Executed | ActionStatus::Interrupted => {
                    return Err(
                        "Status not handled by concierge for tool registry execution".to_string(),
                    );
                }
            }
        }
        Err(e) => {
            error!(
                "Security gate error for registry tool '{}': {}",
                tool_name, e
            );
            return Err(format!("Security gate error: {}", e));
        }
    }

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

/// Resolve and execute a tool call via the tool registry, falling back to
/// the built-in tool dispatch for backward compatibility.
async fn execute_tool_call(
    function_name: &str,
    args: &[String],
    store: Arc<std::sync::Mutex<Store>>,
) -> Result<String, String> {
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
    match function_name {
        "run_command" => {
            if args.len() < 2 {
                return Err(
                    "Error: run_command requires at least 2 arguments (tool and args)".to_string(),
                );
            }
            let tool = &args[0];
            let command_args = &args[1..];

            // Security layer: check command before execution with provenance.
            let guard_root = std::env::var("MIRROR_GUARD_ROOT").unwrap_or_else(|_| {
                std::env::var("CRABJAR_ROOT").unwrap_or_else(|_| ".".to_string())
            });

            let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(
                format!("{}/mirror.db", guard_root),
            ))
            .unwrap_or_else(|_| {
                warn!("Failed to open guard DB, using in-memory fallback");
                crabjar_guard::GuardDb::open(":memory:").unwrap()
            });

            let gate = ExecutionGate::new(&guard_db, false, &guard_root);

            // Construct scope for this orchestrator instance
            let actor_scope = crabjar_guard::Scope::project("orchestrator");
            let target_scope = actor_scope.clone();
            let cross_scope_auth =
                crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);

            let mut concierge = GateConcierge::new().with_db(guard_db.clone());

            match gate.check(GateContext {
                action_type: "tool_call",
                command: tool,
                args: command_args.to_vec(),
                trust_layer: 2,
                confidence: crabjar_guard::TrustScore::new(0.5),
                source_event_id: Some("orchestrator-tc"),
                can_interrupt: true,
                pid: None,
                scope: Some(actor_scope.clone()),
                target_scope: Some(target_scope.clone()),
                cross_scope_auth,
                domains: vec![], // tool calls: no known domains at exec level
                context_budget: None,
                context_fragment_tokens: None,
            }) {
                Ok(result) => {
                    let (status, pending_entry, interrupted_entry) = concierge.enforce(
                        result,
                        "tool_call",
                        tool,
                        command_args,
                        2,
                        0.5,
                        Some("orchestrator-tc".to_string()),
                    );

                    match status {
                        ActionStatus::TrustApproved => {
                            info!(
                                "Gate concierge: Proceed — {} with args {:?}",
                                tool, command_args
                            );
                        }
                        ActionStatus::Pending => {
                            if let Some(ref entry) = pending_entry {
                                info!(
                                    gate_result_id = %entry.gate_result_id,
                                    pending_id = %entry.id,
                                    "Gate concierge: Pending → PendingQueue — queued for review"
                                );
                            }
                            return Err(format!(
                                "Pending: queued for review (pending_id: {})",
                                pending_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Denied => {
                            if let Some(ref entry) = interrupted_entry {
                                info!(
                                    gate_result_id = %entry.gate_result_id,
                                    interrupted_id = %entry.id,
                                    reason = %entry.reason,
                                    "Gate concierge: Interrupted → InterruptedLog"
                                );
                            }
                            return Err(format!(
                                "Interrupted: {} (interrupted_id: {})",
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.reason.clone())
                                    .unwrap_or_default(),
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Executed | ActionStatus::Interrupted => {
                            return Err("Status not handled by concierge".to_string());
                        }
                    }
                }
                Err(e) => {
                    error!("Security gate error: {}", e);
                    return Err(format!("Security gate error: {}", e));
                }
            }

            let mut child = match tokio::process::Command::new(tool)
                .args(command_args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    return Err(format!("Error spawning command: {}", e));
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
                            Ok(None) => {},
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
                            Ok(None) => {},
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
        "search_logs" => {
            // Security layer: check command before execution with provenance.
            let guard_root = std::env::var("MIRROR_GUARD_ROOT").unwrap_or_else(|_| {
                std::env::var("CRABJAR_ROOT").unwrap_or_else(|_| ".".to_string())
            });

            let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(
                format!("{}/mirror.db", guard_root),
            ))
            .unwrap_or_else(|_| {
                warn!("Failed to open guard DB for search_logs, using in-memory fallback");
                crabjar_guard::GuardDb::open(":memory:").unwrap()
            });

            let gate = ExecutionGate::new(&guard_db, false, &guard_root);

            // Construct scope for this orchestrator instance
            let actor_scope = crabjar_guard::Scope::project("orchestrator");
            let target_scope = actor_scope.clone();
            let cross_scope_auth =
                crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);

            let mut concierge = GateConcierge::new().with_db(guard_db.clone());

            match gate.check(GateContext {
                action_type: "tool_call",
                command: "search_logs",
                args: args.to_vec(),
                trust_layer: 2,
                confidence: crabjar_guard::TrustScore::new(0.5),
                source_event_id: Some("orchestrator-sl"),
                can_interrupt: true,
                pid: None,
                scope: Some(actor_scope.clone()),
                target_scope: Some(target_scope.clone()),
                cross_scope_auth,
                domains: vec![],
                context_budget: None,
                context_fragment_tokens: None,
            }) {
                Ok(result) => {
                    let (status, pending_entry, interrupted_entry) = concierge.enforce(
                        result,
                        "tool_call",
                        "search_logs",
                        args,
                        2,
                        0.5,
                        Some("orchestrator-sl".to_string()),
                    );

                    match status {
                        ActionStatus::TrustApproved => {}
                        ActionStatus::Pending => {
                            return Err(format!(
                                "Pending: queued for review (pending_id: {})",
                                pending_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Denied => {
                            return Err(format!(
                                "Interrupted: {} (interrupted_id: {})",
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.reason.clone())
                                    .unwrap_or_default(),
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Executed | ActionStatus::Interrupted => {
                            return Err("Status not handled by concierge".to_string());
                        }
                    }
                }
                Err(e) => {
                    error!("Security gate error for search_logs: {}", e);
                    return Err(format!("Security gate error: {}", e));
                }
            }

            let search_req: SearchLogsRequest =
                match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
                    Ok(req) => req,
                    Err(e) => {
                        return Err(format!("Error parsing search_logs arguments: {}", e));
                    }
                };

            let events = match store
                .lock()
                .unwrap()
                .search_content(&search_req.term, search_req.limit.map(|l| l as usize))
            {
                Ok(events) => events,
                Err(StoreError::DatabaseError(e)) => {
                    return Err(format!("Error searching events: {e}"));
                }
                Err(StoreError::JsonError(e)) => {
                    return Err(format!("Error serializing search results: {e}"));
                }
                Err(StoreError::Internal(e)) => {
                    return Err(format!("Error searching events: {e}"));
                }
            };

            let mut output = String::new();
            output.push_str(&format!(
                "Found {} entries matching '{}':\n",
                events.len(),
                search_req.term
            ));

            for event in events.iter().take(search_req.limit.unwrap_or(10) as usize) {
                output.push_str(&format!(
                    "[id:{}] {}\n",
                    event.id,
                    &event.content[..event.content.len().min(200)]
                ));
            }

            Ok(output)
        }
        "recent_events" => {
            // Security layer: check command before execution with provenance.
            let guard_root = std::env::var("MIRROR_GUARD_ROOT").unwrap_or_else(|_| {
                std::env::var("CRABJAR_ROOT").unwrap_or_else(|_| ".".to_string())
            });

            let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(
                format!("{}/guard.db", guard_root),
            ))
            .unwrap_or_else(|_| {
                warn!("Failed to open guard DB for recent_events, using in-memory fallback");
                crabjar_guard::GuardDb::open(":memory:").unwrap()
            });

            let gate = ExecutionGate::new(&guard_db, false, &guard_root);

            // Construct scope for this orchestrator instance
            let actor_scope = crabjar_guard::Scope::project("orchestrator");
            let target_scope = actor_scope.clone();
            let cross_scope_auth =
                crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);

            let mut concierge = GateConcierge::new().with_db(guard_db.clone());

            match gate.check(GateContext {
                action_type: "tool_call",
                command: "recent_events",
                args: args.to_vec(),
                trust_layer: 2,
                confidence: crabjar_guard::TrustScore::new(0.5),
                source_event_id: Some("orchestrator-re"),
                can_interrupt: true,
                pid: None,
                scope: Some(actor_scope.clone()),
                target_scope: Some(target_scope.clone()),
                cross_scope_auth,
                domains: vec![],
                context_budget: None,
                context_fragment_tokens: None,
            }) {
                Ok(result) => {
                    let (status, pending_entry, interrupted_entry) = concierge.enforce(
                        result,
                        "tool_call",
                        "recent_events",
                        args,
                        2,
                        0.5,
                        Some("orchestrator-re".to_string()),
                    );

                    match status {
                        ActionStatus::TrustApproved => {}
                        ActionStatus::Pending => {
                            return Err(format!(
                                "Pending: queued for review (pending_id: {})",
                                pending_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Denied => {
                            return Err(format!(
                                "Interrupted: {} (interrupted_id: {})",
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.reason.clone())
                                    .unwrap_or_default(),
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Executed | ActionStatus::Interrupted => {
                            return Err("Status not handled by concierge".to_string());
                        }
                    }
                }
                Err(e) => {
                    error!("Security gate error for recent_events: {}", e);
                    return Err(format!("Security gate error: {}", e));
                }
            }

            let recent_req: RecentEventsRequest =
                match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
                    Ok(req) => req,
                    Err(e) => {
                        return Err(format!("Error parsing recent_events arguments: {}", e));
                    }
                };

            let events = match store.lock().unwrap().events(recent_req.limit as usize) {
                Ok(events) => events,
                Err(StoreError::DatabaseError(e)) => {
                    return Err(format!("Error fetching recent events: {e}"));
                }
                Err(StoreError::JsonError(e)) => {
                    return Err(format!("Error serializing events: {e}"));
                }
                Err(StoreError::Internal(e)) => {
                    return Err(format!("Error fetching recent events: {e}"));
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

            Ok(output)
        }
        "by_source" => {
            // Security layer: check command before execution with provenance.
            let guard_root = std::env::var("MIRROR_GUARD_ROOT").unwrap_or_else(|_| {
                std::env::var("CRABJAR_ROOT").unwrap_or_else(|_| ".".to_string())
            });

            let guard_db = crabjar_guard::GuardDb::open(crabjar_guard::GuardDb::from_mirror_path(
                format!("{}/guard.db", guard_root),
            ))
            .unwrap_or_else(|_| {
                warn!("Failed to open guard DB for by_source, using in-memory fallback");
                crabjar_guard::GuardDb::open(":memory:").unwrap()
            });

            let gate = ExecutionGate::new(&guard_db, false, &guard_root);

            // Construct scope for this orchestrator instance
            let actor_scope = crabjar_guard::Scope::project("orchestrator");
            let target_scope = actor_scope.clone();
            let cross_scope_auth =
                crabjar_guard::CrossScopeAuth::auto_for_scopes(&actor_scope, &target_scope);

            let mut concierge = GateConcierge::new().with_db(guard_db.clone());

            match gate.check(GateContext {
                action_type: "tool_call",
                command: "by_source",
                args: args.to_vec(),
                trust_layer: 2,
                confidence: crabjar_guard::TrustScore::new(0.5),
                source_event_id: Some("orchestrator-bs"),
                can_interrupt: true,
                pid: None,
                scope: Some(actor_scope.clone()),
                target_scope: Some(target_scope.clone()),
                cross_scope_auth,
                domains: vec![],
                context_budget: None,
                context_fragment_tokens: None,
            }) {
                Ok(result) => {
                    let (status, pending_entry, interrupted_entry) = concierge.enforce(
                        result,
                        "tool_call",
                        "by_source",
                        args,
                        2,
                        0.5,
                        Some("orchestrator-bs".to_string()),
                    );

                    match status {
                        ActionStatus::TrustApproved => {}
                        ActionStatus::Pending => {
                            return Err(format!(
                                "Pending: queued for review (pending_id: {})",
                                pending_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Denied => {
                            return Err(format!(
                                "Interrupted: {} (interrupted_id: {})",
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.reason.clone())
                                    .unwrap_or_default(),
                                interrupted_entry
                                    .as_ref()
                                    .map(|e| e.id.clone())
                                    .unwrap_or_default()
                            ));
                        }
                        ActionStatus::Executed | ActionStatus::Interrupted => {
                            return Err("Status not handled by concierge".to_string());
                        }
                    }
                }
                Err(e) => {
                    error!("Security gate error for by_source: {}", e);
                    return Err(format!("Security gate error: {}", e));
                }
            }

            let source_req: BySourceRequest =
                match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
                    Ok(req) => req,
                    Err(e) => {
                        return Err(format!("Error parsing by_source arguments: {}", e));
                    }
                };

            let events = match store.lock().unwrap().query(
                &[&source_req.source],
                source_req.limit.unwrap_or(50) as usize,
                "",
                "",
                "",
            ) {
                Ok(events) => events,
                Err(StoreError::DatabaseError(e)) => {
                    return Err(format!("Error fetching events by source: {e}"));
                }
                Err(StoreError::JsonError(e)) => {
                    return Err(format!("Error serializing events: {e}"));
                }
                Err(StoreError::Internal(e)) => {
                    return Err(format!("Error fetching events by source: {e}"));
                }
            };

            let mut output = String::new();
            output.push_str(&format!("Events from source '{}':\n", source_req.source));

            for event in events.iter() {
                output.push_str(&format!(
                    "[id:{}] {}\n",
                    event.id,
                    &event.content[..event.content.len().min(200)]
                ));
            }

            Ok(output)
        }
        _ => Ok(format!("Unknown tool: {}", function_name)),
    }
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

    // Shared state across request handlers
    let state = AppState {
        store: Arc::new(std::sync::Mutex::new(store)),
        events_db_path,
        guard_root,
        backend: Arc::new(Mutex::new(backend)),
        actor_scope,
        target_scope,
    };

    // Define the Axum router with SSE and JSON endpoints.
    let app = Router::new()
        .route("/acp/run", post(handle_run))
        .route("/acp/prompt", post(handle_prompt))
        .route("/acp/chat", post(handle_chat))
        .route("/acp/search_logs", post(search_logs))
        .route("/acp/recent_events", post(recent_events))
        .route("/acp/by_source", post(by_source))
        .with_state(state)
        .layer(CorsLayer::permissive());

    // Bind to localhost:3000.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("ACP Orchestrator listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
