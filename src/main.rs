use clap::Parser;
use crabjar_lib::{Cli, CliCommand, StateCommand, WorkspaceCommand};
use serde_json::json;

mod bitwarden;
mod crabjar_config;
mod doctor;
mod dotfile_manager;
mod knowledge_store;
mod metrics;
mod project_loader;
mod tool_registry_cli;

use bitwarden::commands::handle_bitwarden_command;
use crabjar_lib::{
    BackendCommand, BitwardenCommand, DoctorCommand, DotfileCommand, GuardCommand,
    KnowledgeCommand, MetricsCommand, ToolCommand,
};
use doctor::handle_doctor_command;
use dotfile_manager::DotfileManager;
use knowledge_store::KnowledgeBridge;
use knowledge_store::commands::KnowledgeCommandExt;
use metrics::{run_module_sizes, run_test_count};
use project_loader::ProjectLoader;
use tool_registry_cli::handle_tool_command;

fn is_help_request(args: &[String]) -> bool {
    args.iter()
        .any(|a| a == "--help" || a == "-h" || a == "help")
}

/// Main CLI entry point for CrabJar
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if is_help_request(&args) {
        print_json(&usage_response(true));
        return;
    }

    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(err) => {
            print_json(&error_response(&err.to_string(), true));
            std::process::exit(1);
        }
    };

    let response = match cli.command {
        Some(CliCommand::Help) => usage_response(true),
        Some(CliCommand::State { command }) => handle_state_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Knowledge { command }) => handle_knowledge_command(command)
            .await
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Dotfile { command }) => handle_dotfile_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Workspace {
            command: WorkspaceCommand::Status,
        }) => handle_workspace_status().await,
        Some(CliCommand::Guard { command }) => handle_guard_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Exec {
            command,
            args,
            cwd,
            reason,
            dry_run,
        }) => handle_exec(&command, &args, &cwd, &reason, dry_run)
            .await
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Bitwarden { command }) => handle_bitwarden_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Doctor { command }) => handle_doctor_command(command)
            .await
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Backend { command }) => handle_backend_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Tool { command }) => handle_tool_command(command)
            .await
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Metrics { command }) => handle_metrics_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        None => {
            print_json(&error_response("missing command", true));
            std::process::exit(1);
        }
    };

    let exit_code = response
        .get("success")
        .and_then(|value| value.as_bool())
        .map(|success| if success { 0 } else { 1 })
        .unwrap_or(1);

    print_json(&response);
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn print_json(response: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(response).unwrap_or_else(|_| {
            "{\"success\":false,\"error\":\"failed to serialize response\"}".to_string()
        })
    );
}

/// Handle state-docs commands (SQLite-backed via agent_context::state_docs)
fn handle_state_command(
    command: StateCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        StateCommand::Index { docs_dir, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let count = agent_context::state_docs::indexer::index_all_docs(
                &conn,
                std::path::Path::new(&docs_dir),
            )?;
            Ok(json!({
                "success": true,
                "message": format!("indexed {} state-docs", count),
                "payload": {
                    "count": count,
                    "docs_dir": docs_dir,
                    "db_path": db_path,
                }
            }))
        }
        StateCommand::Show { doc_name, zoom } => {
            let conn = rusqlite::Connection::open("state-docs.db")?;
            agent_context::state_docs::migrate(&conn)?;
            // Strip .md extension to match how the indexer stores doc names (from frontmatter `name:`)
            let lookup_name = doc_name.strip_suffix(".md").unwrap_or(&doc_name);
            let renderer = agent_context::state_docs::Renderer::new(&conn);
            let (markdown, metadata) = renderer.render_doc(lookup_name, zoom)?;
            Ok(json!({
                "success": true,
                "message": format!("rendered {} at zoom level {}", doc_name, zoom),
                "payload": {
                    "doc": doc_name,
                    "zoom": zoom,
                    "markdown": markdown,
                    "metadata": metadata,
                }
            }))
        }
        StateCommand::Query {
            doc_name,
            section,
            keyword,
            db_path,
        } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            let querier = agent_context::state_docs::StateDocQuerier::new(
                conn,
                std::path::PathBuf::from(&db_path),
            );
            if let Some(section) = section {
                let result = querier.query_by_section(&doc_name, &section);
                Ok(json!({
                    "success": true,
                    "message": format!("queried section '{}' in {}", section, doc_name),
                    "payload": result,
                }))
            } else if let Some(keyword) = keyword {
                let result = querier.query_by_keyword(&doc_name, &keyword);
                Ok(json!({
                    "success": true,
                    "message": format!("searched keyword '{}' in {}", keyword, doc_name),
                    "payload": result,
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": "must provide --section or --keyword",
                }))
            }
        }
        StateCommand::List { db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT doc_name, description, last_modified, line_count, checksum FROM doc_metadata ORDER BY last_modified DESC"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(json!({
                    "name": row.get::<_, String>(0)?,
                    "description": row.get::<_, String>(1)?,
                    "last_modified": row.get::<_, String>(2)?,
                    "line_count": row.get::<_, i64>(3)?,
                    "checksum": row.get::<_, String>(4)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("listed {} state-docs", results.len()),
                "payload": {
                    "docs": results,
                }
            }))
        }
        StateCommand::Confidence { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT what_captured, what_missed, assumptions, blind_spots, stale_after FROM confidence WHERE doc_id = ?1"
            )?;
            let row = stmt
                .query_row(rusqlite::params![doc_name], |row| {
                    Ok(json!({
                        "what_captured": row.get::<_, String>(0)?,
                        "what_missed": row.get::<_, String>(1)?,
                        "assumptions": row.get::<_, String>(2)?,
                        "blind_spots": row.get::<_, String>(3)?,
                        "stale_after": row.get::<_, String>(4)?,
                    }))
                })
                .map_err(|e| {
                    if e == rusqlite::Error::QueryReturnedNoRows {
                        format!("no confidence assessment for {}", doc_name)
                    } else {
                        e.to_string()
                    }
                })?;
            Ok(json!({
                "success": true,
                "message": format!("retrieved confidence for {}", doc_name),
                "payload": {
                    "doc": doc_name,
                    "confidence": row,
                }
            }))
        }
        StateCommand::Annotations { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT line, kind, message, author, status, created_at FROM annotations WHERE doc_id = ?1 ORDER BY line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "line": row.get::<_, i64>(0)?,
                    "kind": row.get::<_, String>(1)?,
                    "message": row.get::<_, String>(2)?,
                    "author": row.get::<_, String>(3)?,
                    "status": row.get::<_, String>(4)?,
                    "created_at": row.get::<_, String>(5)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            let open_count = results
                .iter()
                .filter(|a| a.get("status").and_then(|s| s.as_str()) == Some("open"))
                .count();
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} annotations for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "annotations": results,
                    "open_count": open_count,
                }
            }))
        }
        StateCommand::Tables { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT start_line, end_line, headers, rows FROM tables WHERE doc_id = ?1 ORDER BY start_line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "start_line": row.get::<_, i64>(0)?,
                    "end_line": row.get::<_, i64>(1)?,
                    "headers": row.get::<_, String>(2)?,
                    "rows": row.get::<_, String>(3)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} tables for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "tables": results,
                }
            }))
        }
        StateCommand::CodeBlocks { doc_name, db_path } => {
            let conn = rusqlite::Connection::open(&db_path)?;
            agent_context::state_docs::migrate(&conn)?;
            let mut stmt = conn.prepare(
                "SELECT start_line, end_line, language, content_hash FROM code_blocks WHERE doc_id = ?1 ORDER BY start_line ASC"
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_name], |row| {
                Ok(json!({
                    "start_line": row.get::<_, i64>(0)?,
                    "end_line": row.get::<_, i64>(1)?,
                    "language": row.get::<_, String>(2)?,
                    "line_count": row.get::<_, i64>(3)?,
                }))
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(json!({
                "success": true,
                "message": format!("retrieved {} code blocks for {}", results.len(), doc_name),
                "payload": {
                    "doc": doc_name,
                    "code_blocks": results,
                }
            }))
        }
    }
}

fn handle_dotfile_command(
    command: DotfileCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let manager = DotfileManager::new(project_root);

    match command {
        DotfileCommand::Promote { path } => manager.propose(&path, &path),
    }
}

/// Handle knowledge commands
async fn handle_knowledge_command(
    command: KnowledgeCommand,
) -> Result<serde_json::Value, agent_context::Error> {
    let project_root = std::env::current_dir()
        .map_err(|err| agent_context::Error::Io(std::io::Error::other(err.to_string())))?;

    // Build guard DB path for gate
    let guard_db_path = project_root.join("guard/guard.db");
    let guard_db = crabjar_guard::GuardDb::open(&guard_db_path).unwrap_or_else(|_| {
        let temp_dir = tempfile::tempdir().unwrap();
        crabjar_guard::GuardDb::open(temp_dir.path().join("guard.db")).unwrap()
    });

    let bridge = KnowledgeBridge::new(
        project_root
            .join("memory/knowledge.db")
            .to_string_lossy()
            .as_ref(),
        project_root,
        None,
    )?
    .with_guard_db(guard_db);
    command.execute(&bridge).await
}

/// Handle workspace status
async fn handle_workspace_status() -> serde_json::Value {
    let project_root = std::env::current_dir().ok();
    let loader = ProjectLoader::new();

    if let Some(root) = project_root {
        let mut loader = loader;
        if loader.load_from_directory(&root).await.is_ok()
            && let Some(config) = loader.get_current_config()
        {
            return json!({
                "success": true,
                "workspace": {
                    "name": config.name,
                    "description": config.description,
                    "declared_tools": config.tools.len(),
                    "tool_execution_enabled": config.tool_execution_enabled,
                    "user_dinit_socket": config.user_dinit_socket,
                }
            });
        }
    }

    json!({
        "success": true,
        "workspace": null,
    })
}

/// Error response helper
async fn handle_exec(
    command: &str,
    args: &[String],
    cwd: &str,
    reason: &str,
    dry_run: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;

    // Config check: tool_execution_enabled must be true
    let loader = ProjectLoader::new();
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
        project_root.to_string_lossy().into_owned()
    } else {
        cwd.to_string()
    };

    // Guard layer: gate check before execution
    let guard_db = crabjar_guard::GuardDb::open(project_root.join("guard/guard.db"))
        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(":memory:").expect("guard db fallback"));

    let gate = crabjar_guard::ExecutionGate::new(&guard_db, dry_run, &project_root);

    // Build scope from project root for scope isolation
    let project_scope = crabjar_guard::Scope::project(
        project_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "default".to_string()),
    );

    // Auto-construct CrossScopeAuth if scopes differ (same-scope → None, no-op)
    let cross_scope_auth = crabjar_guard::CrossScopeAuth::auto_for_scopes(
        &project_scope,
        &project_scope,
    );

    let gate_result = gate.check(crabjar_guard::GateContext {
        action_type: "exec",
        command,
        args: args.to_vec(),
        trust_layer: 3,
        confidence: crabjar_guard::TrustScore::new(0.9),
        source_event_id: Some(reason),
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
    let mut concierge = crabjar_guard::GateConcierge::default();
    let (status, pending_entry, interrupted_entry) = concierge.enforce(
        gate_result,
        "exec",
        command,
        args,
        3,
        0.9,
        Some(reason.to_string()),
    );

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
            if let Some(ref entry) = pending_entry {
                guard_db.persist_pending_queue_entry(entry)?;
            }
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
            // Telemetry layer: persistent flight recorder
            let flight_db_path = project_root.join("telemetry/flight.db");
            let flight_conn = rusqlite::Connection::open(&flight_db_path)?;
            let mut flight_recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
                &flight_conn,
                "exec-session",
            );
            flight_recorder.init()?;

            // Tool registry: discover and validate tools before execution
            let tool_registry_path = project_root.join("tool_registry/tool_registry.db");
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
                registry.discover_tools("cli", &project_root).unwrap_or_default()
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

            let cmd_id = flight_recorder
                .execute_command(command, args, &effective_cwd, reason)
                .await?;

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);
            let receipt = records
                .first()
                .map(|r| r.receipt.clone())
                .unwrap_or_default();

            let git_dirty = flight_recorder.capture_git_dirty(&effective_cwd).await?;
            let git_diff = flight_recorder.capture_git_diff(&effective_cwd).await?;

            // Outcome layer: record in GuardDb action_outcomes
            let outcome_id = crabjar_guard::GuardDb::open(&flight_db_path)
                .ok()
                .map(|db| {
                    let conn = db.conn();
                    let id = uuid::Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO action_outcomes (id, action_id, success, exit_code, output_hash, confidence_delta, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())",
                        rusqlite::params![
                            id,
                            cmd_id,
                            exit_code != -1,
                            exit_code,
                            git_diff,
                            0.02,
                        ],
                    )
                })
                .map(|_| uuid::Uuid::new_v4().to_string());

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

            Ok(json!({
                "success": true,
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

/// Execute dinitctl command, routing through crabjar-dinit instance when configured.
fn execute_via_dinitctl(
    socket_path: &Option<String>,
    args: &[String],
    cwd: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let effective_socket = match socket_path {
        Some(s) => s.clone(),
        None => "/var/run/dinit.sock".to_string(),
    };

    // Build dinitctl args with crabjar-dinit socket
    let mut dinit_args: Vec<String> = vec!["-p".to_string(), effective_socket.clone()];
    dinit_args.extend(args.iter().cloned());
    let dinit_args: Vec<&str> = dinit_args.iter().map(|s| s.as_str()).collect();

    let output = std::process::Command::new("dinitctl")
        .args(&dinit_args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("dinitctl failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(stderr.trim().to_string().into());
    }

    // Parse exit code from output if present
    let exit_code = stdout
        .lines()
        .find_map(|line| line.trim().parse::<i32>().ok())
        .unwrap_or(0);

    Ok(exit_code)
}

/// Handle guard commands
fn handle_guard_command(
    command: GuardCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let guard_db = crabjar_guard::GuardDb::open(project_root.join("guard/guard.db"))
        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(":memory:").expect("guard db fallback"));

    match command {
        GuardCommand::Queue { status, limit } => {
            let requests = guard_db.read_action_requests(Some(&status), limit)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "queue": {
                        "status": status,
                        "entries": requests,
                    },
                },
            }))
        }
        GuardCommand::Approve { action_id } => {
            guard_db
                .update_action_status(&action_id, crabjar_guard::ActionStatus::TrustApproved)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "approve": {
                        "action_id": action_id,
                        "status": "trust-approved",
                    },
                },
            }))
        }
        GuardCommand::Reject { action_id, reason } => {
            guard_db.update_action_status(&action_id, crabjar_guard::ActionStatus::Denied)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "reject": {
                        "action_id": action_id,
                        "reason": reason,
                        "status": "denied",
                    },
                },
            }))
        }
        GuardCommand::Interrupted { limit: _ } => {
            let entries = guard_db.read_interrupted_log()?;
            Ok(json!({
                "success": true,
                "guard": {
                    "interrupted": {
                        "entries": entries,
                    },
                },
            }))
        }
        GuardCommand::Provenance { source_event_id } => {
            let exists = guard_db.verify_provenance(&source_event_id)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "provenance": {
                        "source_event_id": source_event_id,
                        "exists": exists,
                    },
                },
            }))
        }
        GuardCommand::Grant {
            pid,
            trust_layer,
            auto_grant,
        } => {
            guard_db.grant_pid_trust(pid, trust_layer, auto_grant)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "grant": {
                        "pid": pid,
                        "trust_layer": trust_layer,
                        "auto_grant": auto_grant,
                    },
                },
            }))
        }
        GuardCommand::Revoke { pid } => {
            let result = guard_db.revoke_pid_trust(pid)?;
            Ok(json!({
                "success": true,
                "guard": {
                    "revoke": {
                        "pid": pid,
                        "old_layer": result.map(|(l, _)| l),
                        "status": "revoked",
                    },
                },
            }))
        }
        GuardCommand::Resolution {
            limit,
            effective_layer,
        } => {
            let entries = guard_db.list_trust_resolutions(effective_layer, limit)?;
            let items: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "action_id": e.action_id,
                        "requested_layer": e.requested_layer,
                        "requested_confidence": e.requested_confidence,
                        "requested_source": e.requested_source,
                        "effective_layer": e.effective_layer,
                        "effective_confidence": e.effective_confidence,
                        "effective_by": e.effective_by,
                        "scope_actor": e.scope_actor,
                        "scope_target": e.scope_target,
                        "applied_policies": e.applied_policies,
                        "resolved_at": e.resolved_at,
                    })
                })
                .collect();
            Ok(json!({
                "success": true,
                "guard": {
                    "resolution": {
                        "limit": limit,
                        "effective_layer_filter": effective_layer,
                        "entries": items,
                        "total": items.len(),
                    },
                },
            }))
        }
    }
}

fn error_response(message: &str, show_usage: bool) -> serde_json::Value {
    let mut response = json!({
        "success": false,
        "error": message,
    });

    if show_usage {
        response["usage"] = json!(usage_lines());
    }

    response
}

/// Usage response helper
fn usage_response(show_usage: bool) -> serde_json::Value {
    if show_usage {
        json!({
            "success": true,
            "error": null,
            "usage": usage_lines(),
        })
    } else {
        json!({
            "success": true,
            "error": null,
        })
    }
}

fn usage_lines() -> &'static [&'static str] {
    &[
        "crabjar state list",
        "crabjar state show <doc>",
        "crabjar state annotate <doc> <message>",
        "crabjar state question <doc> <message>",
        "crabjar state resolve <doc> <id>",
        "crabjar knowledge <subcommand>",
        "crabjar dotfile <subcommand>",
        "crabjar workspace status",
        "crabjar guard grant --pid=<pid>",
        "crabjar guard revoke --pid=<pid>",
        "crabjar bitwarden <subcommand>",
        "crabjar doctor check",
        "crabjar backend set --backend=<lm-studio|native>",
        "crabjar backend get",
    ]
}

/// Handle backend commands
fn handle_backend_command(command: BackendCommand) -> Result<serde_json::Value, String> {
    match command {
        BackendCommand::Set { backend } => {
            // Validate backend type
            match backend.as_str() {
                "lm-studio" | "native" => {
                    // Update environment variable for the orchestrator
                    unsafe {
                        std::env::set_var("INFERENCE_BACKEND", &backend);
                    }
                    Ok(json!({
                        "success": true,
                        "message": format!("Inference backend set to: {}", backend),
                    }))
                }
                _ => Err(format!(
                    "Invalid backend: {}. Use 'lm-studio' or 'native'.",
                    backend
                )),
            }
        }
        BackendCommand::Get => {
            let current_backend =
                std::env::var("INFERENCE_BACKEND").unwrap_or_else(|_| "lm-studio".to_string());
            Ok(json!({
                "success": true,
                "current_backend": current_backend,
            }))
        }
    }
}

/// Handle metrics commands
fn handle_metrics_command(command: MetricsCommand) -> Result<serde_json::Value, String> {
    match command {
        MetricsCommand::All => {
            let tests = run_test_count();
            let modules = run_module_sizes();
            Ok(json!({
                "success": true,
                "metrics": {
                    "tests": tests,
                    "modules": modules,
                },
                "usage": [
                    "crabjar metrics all",
                    "crabjar metrics tests",
                    "crabjar metrics modules",
                ],
            }))
        }
        MetricsCommand::Tests => Ok(run_test_count()),
        MetricsCommand::Modules => Ok(run_module_sizes()),
    }
}
