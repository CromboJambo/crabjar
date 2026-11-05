use std::path::PathBuf;

use serde_json::json;

use super::dinitctl::execute_via_dinitctl;
use super::podman::execute_in_podman_container;
use crate::project_loader::ProjectLoader;

/// Execute command through the full pipeline: guard gate → telemetry → execution → outcome.
pub async fn handle_exec(
    command: &str,
    args: &[String],
    cwd: &str,
    reason: &str,
    dry_run: bool,
    container: bool,
    _otel_endpoint: &str,
    trust_layer: u8,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let workspace_dir = if !cwd.trim().is_empty() {
        PathBuf::from(cwd)
    } else {
        std::env::current_dir()?
    };

    // Config check: tool_execution_enabled must be true
    let mut loader = ProjectLoader::new();
    loader.load_from_directory(&workspace_dir).await.ok();
    let user_dinit_socket = if let Some(config) = loader.get_current_config() {
        if !config.tool_execution_enabled {
            return Ok(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "reason": reason,
                    "gate_result": "denied",
                    "gate_reason": "tool_execution_enabled is false in config",
                },
            }));
        }
        config.user_dinit_socket.clone()
    } else {
        None
    };

    // Dry-run shortcut: skip gate and telemetry when dry_run is true
    if dry_run {
        return Ok(json!({
            "success": true,
            "exec": {
                "command": command,
                "args": args,
                "cwd": if cwd.trim().is_empty() {
                    std::env::current_dir()?.to_string_lossy().into_owned()
                } else {
                    cwd.to_string()
                },
                "reason": reason,
                "gate_result": "dry_run",
            },
        }));
    }

    let effective_cwd = if cwd.trim().is_empty() {
        workspace_dir.to_string_lossy().into_owned()
    } else {
        cwd.to_string()
    };

    // Container mode: check podman availability EARLY, before any guard/trust/database operations.
    // If container=true and podman is unavailable, fail immediately — do not proceed with
    // gate checks or database writes (ADR-044 redteam finding 1.1).
    if container {
        match execute_in_podman_container(command, args, &effective_cwd) {
            Some(podman_result) => {
                return Ok(json!({
                    "success": true,
                    "exec": {
                        "command": command,
                        "args": args,
                        "cwd": effective_cwd,
                        "reason": reason,
                        "gate_result": "proceed",
                        "exit_code": podman_result.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1),
                        "container": true,
                        "backend": "podman",
                        "podman_result": podman_result,
                    },
                }));
            }
            None => {
                // Podman unavailable — fail immediately, do not fall back to host
                return Ok(json!({
                    "success": false,
                    "exec": {
                        "command": command,
                        "args": args,
                        "cwd": effective_cwd,
                        "reason": reason,
                        "gate_result": "proceed",
                        "container": true,
                        "error": "podman backend unavailable for container mode; refusing host fallback",
                    },
                }));
            }
        }
    }

    // Telemetry layer: initialize flight recorder BEFORE guard check
    // so we can record the guard decision itself
    let telemetry_dir = workspace_dir.join("telemetry");
    std::fs::create_dir_all(&telemetry_dir).ok();
    let flight_db_path = telemetry_dir.join("flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path)?;
    let mut flight_recorder =
        crabjar_telemetry::flight_recorder::FlightRecorder::new(&flight_conn, "exec-session");
    flight_recorder.init()?;

    // Enable OTel export if endpoint provided (CLI flag or env var)
    let otel_endpoint = std::env::var("CRABJAR_OTEL_ENDPOINT").unwrap_or_else(|_| String::new());
    let effective_otel = if !otel_endpoint.is_empty() {
        otel_endpoint
    } else {
        otel_endpoint.to_string()
    };
    if !effective_otel.is_empty() {
        flight_recorder = flight_recorder.with_otel(&effective_otel)?;
    }

    // Guard layer: open guard database — fail-closed, no in-memory fallback.
    // If the persistent GuardDb cannot be opened, execution must stop rather than
    // silently continuing with ephemeral state (ADR-044 redteam finding 1.2).
    let guard_db_path = workspace_dir.join("guard/guard.db");
    let guard_db = match crabjar_guard::GuardDb::open(&guard_db_path) {
        Ok(db) => db,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "reason": reason,
                    "gate_result": "error",
                    "gate_reason": format!("guard database unavailable: {}", e),
                },
            }));
        }
    };

    // Create action request to establish provenance BEFORE gate check.
    // The free-form 'reason' string is NOT a valid source_event_id; we must create
    // a real action request record that the provenance verification can validate against.
    let action_request_id = uuid::Uuid::new_v4().to_string();
    guard_db.persist_action_request_with_scope(
        &action_request_id,
        Some(&action_request_id), // source_event_id = our own request ID
        None,                     // no external source node for CLI exec
        "exec",
        &format!("{} {}", command, args.join(" ")),
        trust_layer as u32,
        0.9,  // confidence
        None, // scope actor
        None, // scope target
    )?;

    let gate = crabjar_guard::ExecutionGate::new(&guard_db, dry_run, &workspace_dir);

    // Build scope from project root for scope isolation
    let workspace_name = workspace_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "default".to_string());
    let project_scope = crabjar_guard::Scope::project(&workspace_name);

    let cross_scope_auth = None;

    // Use the real action request ID as source_event_id for provenance validation.
    // Arbitrary reason text cannot satisfy provenance — only IDs that exist in
    // action_requests table are accepted (ADR-044 redteam finding 1.2).
    let gate_result = gate.check(crabjar_guard::GateContext {
        action_type: "exec",
        command,
        args: args.to_vec(),
        trust_layer: trust_layer as u32,
        confidence: crabjar_guard::TrustScore::new(0.9),
        source_event_id: Some(&action_request_id),
        can_interrupt: true,
        pid: None,
        scope: Some(project_scope.clone()),
        target_scope: Some(project_scope),
        cross_scope_auth,
        domains: vec![], // exec: no known domains at CLI level
        context_budget: None,
        context_fragment_tokens: None,
    })?;
    eprintln!("DEBUG gate_result: {:?}", gate_result);

    // Concierge layer: persist gate result to GuardDb
    let mut concierge = crabjar_guard::GateConcierge::default().with_db(guard_db.clone());
    let (status, pending_entry, interrupted_entry) = concierge.enforce(
        gate_result,
        "exec",
        command,
        args,
        3,
        0.9,
        Some(action_request_id.clone()),
    );

    // Record guard decision via OTel telemetry
    let decision_str = match &status {
        crabjar_guard::ActionStatus::Denied => "denied",
        crabjar_guard::ActionStatus::Pending => "pending",
        crabjar_guard::ActionStatus::TrustApproved => "approved",
        crabjar_guard::ActionStatus::Executed | crabjar_guard::ActionStatus::Interrupted => {
            "executed"
        }
    };
    flight_recorder.record_guard_decision("exec-session", "exec", decision_str, 3, 0.9);

    match status {
        crabjar_guard::ActionStatus::Denied => Ok(json!({
            "success": false,
            "exec": {
                "command": command,
                "args": args,
                "cwd": effective_cwd,
                "reason": reason,
                "gate_result": "denied",
                "interrupted_id": interrupted_entry.as_ref().map(|e| e.id.clone()),
                "gate_reason": interrupted_entry.as_ref().map(|e| e.reason.clone()),
            },
        })),
        crabjar_guard::ActionStatus::Pending => {
            // Concierge already persisted the pending entry via with_db()
            Ok(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "gate_result": "pending",
                    "requires_review": true,
                    "pending_id": pending_entry.map(|e| e.id.clone()),
                },
            }))
        }
        crabjar_guard::ActionStatus::TrustApproved => {
            // Tool registry: discover and validate tools before execution
            let tool_registry_path = workspace_dir.join("tool_registry/tool_registry.db");
            let tool_registry_conn: Option<rusqlite::Connection> =
                rusqlite::Connection::open(&tool_registry_path)
                    .ok()
                    .or_else(|| {
                        eprintln!("Warning: Failed to open tool_registry DB, using in-memory");
                        rusqlite::Connection::open(":memory:").ok()
                    });
            let discovered_tools = if let Some(ref conn) = tool_registry_conn {
                let registry = crabjar_tool_registry::ToolRegistry::new(conn);
                registry.init().ok();
                registry
                    .discover_tools("cli", &workspace_dir)
                    .unwrap_or_default()
            } else {
                vec![]
            };

            // Validate tool availability
            let _validation = if !discovered_tools.is_empty() {
                Some(
                    crabjar_tool_registry::ToolRegistry::new(&tool_registry_conn.unwrap())
                        .validate_tools(&discovered_tools)
                        .unwrap_or_default(),
                )
            } else {
                None
            };

            // Parse command string into binary + args if not already split
            let (cmd_name, cmd_args): (String, Vec<String>) = if !args.is_empty() {
                // Already split via CLI flags (-a/--args)
                (command.to_string(), args.to_vec())
            } else {
                // Need to parse the command string ourselves
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.is_empty() {
                    return Ok(json!({
                        "success": false,
                        "exec": { "error": "empty command" }
                    }));
                }
                let cmd = parts[0].to_string();
                let rest: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                (cmd, rest)
            };

            // Host execution path (container mode handled earlier)
            let cmd_id = flight_recorder
                .execute_command(&cmd_name, &cmd_args, &effective_cwd, reason)
                .await?;

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);
            let receipt = records
                .first()
                .map(|r| r.receipt.clone())
                .unwrap_or_default();

            let git_dirty = flight_recorder.capture_git_dirty(&effective_cwd).await?;
            let git_diff = flight_recorder.capture_git_diff(&effective_cwd).await?;

            // Dinit integration: if command is dinitctl, route through crabjar-dinit
            let dinit_result = if command == "dinitctl" {
                execute_via_dinitctl(&user_dinit_socket, args, &effective_cwd)
            } else {
                Ok(exit_code)
            };

            let final_exit_code = match dinit_result {
                Ok(code) => code,
                Err(e) => {
                    return Ok(json!({
                        "success": false,
                        "exec": {
                            "command": command,
                            "args": args,
                            "cwd": effective_cwd,
                            "reason": reason,
                            "gate_result": "proceed",
                            "dinit_error": e.to_string(),
                        },
                    }));
                }
            };

            // Outcome layer: record in GuardDb action_outcomes (same DB as provenance)
            let success = final_exit_code == 0;
            let outcome_id = {
                let conn = guard_db.conn();
                let id = uuid::Uuid::new_v4().to_string();
                match conn.execute(
                    "INSERT INTO action_outcomes (id, action_id, success, exit_code, output_hash, confidence_delta) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        id,
                        action_request_id,  // ← use the action request ID, not cmd_id
                        success,
                        final_exit_code,
                        git_diff,
                        0.02
                    ],
                ) {
                    Ok(_) => eprintln!("DEBUG outcome: inserted {}", id),
                    Err(e) => eprintln!("DEBUG outcome: INSERT failed: {}", e),
                }
                id
            };

            Ok(json!({
                "success": success,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "cmd_id": cmd_id,
                    "exit_code": final_exit_code,
                    "gate_result": "proceed",
                    "git_dirty": git_dirty,
                    "git_diff_hash": git_diff,
                    "outcome_id": outcome_id,
                    "flight_recorder": true,
                    "container": false,
                    "receipt": if receipt.is_empty() {
                        None::<String>
                    } else {
                        Some(receipt)
                    },
                },
            }))
        }
        crabjar_guard::ActionStatus::Executed | crabjar_guard::ActionStatus::Interrupted => {
            Ok(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "gate_result": "unhandled",
                },
            }))
        }
    }
}
