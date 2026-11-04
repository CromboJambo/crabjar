use serde_json::json;
use std::path::PathBuf;

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

    // Telemetry layer: initialize flight recorder BEFORE guard check
    // so we can record the guard decision itself
    let flight_db_path = workspace_dir.join("telemetry/flight.db");
    let flight_conn = rusqlite::Connection::open(&flight_db_path)?;
    let mut flight_recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
        &flight_conn,
        "exec-session",
    );
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
        3,                        // trust layer
        0.9,                      // confidence
        None,                     // scope actor
        None,                     // scope target
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
        trust_layer: 3,
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
            // Already persisted by concierge.enforce() via GuardDb
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

            // Container mode: execute ONLY via podman, never fall back to host.
            // If container=true and podman is unavailable, fail — do not silently
            // run on the host (ADR-044 redteam finding 1.1).
            let cmd_id = if container {
                match execute_in_podman_container(command, args, &effective_cwd) {
                    Some(podman_result) => {
                        flight_recorder.record_guard_decision(
                            "exec-session",
                            "exec:container",
                            "approved",
                            3,
                            0.9,
                        );
                        let exit_code = podman_result
                            .get("exit_code")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(-1);
                        // Derive success from exit code (ADR-044 finding 1.3)
                        let success = exit_code == 0;
                        // Return podman result as backend metadata
                        return Ok(json!({
                            "success": success,
                            "exec": {
                                "command": command,
                                "args": args,
                                "cwd": effective_cwd,
                                "reason": reason,
                                "gate_result": "proceed",
                                "exit_code": exit_code,
                                "container": true,
                                "backend": "podman",
                                "podman_result": podman_result,
                            },
                        }));
                    }
                    None => {
                        // Podman unavailable — fail, do not fall back to host
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
            } else {
                // Host execution path (unchanged)
                flight_recorder
                    .execute_command(command, args, &effective_cwd, reason)
                    .await?
            };

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);
            let receipt = records
                .first()
                .map(|r| r.receipt.clone())
                .unwrap_or_default();

            let git_dirty = flight_recorder.capture_git_dirty(&effective_cwd).await?;
            let git_diff = flight_recorder.capture_git_diff(&effective_cwd).await?;

            // Outcome layer: record in the same GuardDb used by the gate (ADR-044 finding 1.4).
            // Return the actual inserted record ID; propagate persistence errors instead of
            // suppressing them with .ok().
            let outcome_id = {
                let id = uuid::Uuid::new_v4().to_string();
                guard_db
                    .conn()
                    .execute(
                        "INSERT INTO action_outcomes (id, action_id, success, exit_code, output_hash, confidence_delta, created_at)\n\
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())",
                        rusqlite::params![
                            id.clone(),
                            cmd_id,
                            exit_code == 0,
                            exit_code,
                            git_diff,
                            0.02,
                        ],
                    )
                    .map_err(|e| format!("failed to persist outcome: {}", e))?;
                id
            };

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

            // Derive success from exit code (ADR-044 finding 1.3): nonzero exit means command failure.
            let success = final_exit_code == 0;
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
