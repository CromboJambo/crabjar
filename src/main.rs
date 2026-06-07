use clap::Parser;
use crabjar_lib::{Cli, CliCommand, StateCommand, WorkspaceCommand};
use serde_json::json;

mod bitwarden;
mod crabjar_config;
mod dinit;
mod dotfile_manager;
mod knowledge_store;
mod project_loader;
mod state_docs;

use crabjar_lib::{
    BitwardenCommand, DoctorCommand, DotfileCommand, GuardCommand, KnowledgeCommand,
    BackendCommand,
};
use dotfile_manager::DotfileManager;
use knowledge_store::KnowledgeBridge;
use knowledge_store::commands::KnowledgeCommandExt;
use project_loader::ProjectLoader;
use state_docs::{AnnotationKind, StateDocsManager};

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

/// Handle state-docs commands
fn handle_state_command(
    command: StateCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let state_docs = StateDocsManager::new(project_root);

    match command {
        StateCommand::List => {
            let docs = state_docs.list_docs()?;
            Ok(json!({
                "success": true,
                "docs": docs,
            }))
        }
        StateCommand::Show { doc } => {
            let view = state_docs.show_doc(&doc)?;
            Ok(json!({
                "success": true,
                "doc": view,
            }))
        }
        StateCommand::Annotate { doc, message } => {
            let entry =
                state_docs.add_annotation(&doc, AnnotationKind::Note, &message, "user", None)?;
            Ok(json!({
                "success": true,
                "annotation": entry,
            }))
        }
        StateCommand::Question { doc, message } => {
            let entry = state_docs.add_annotation(
                &doc,
                AnnotationKind::Question,
                &message,
                "user",
                None,
            )?;
            Ok(json!({
                "success": true,
                "annotation": entry,
            }))
        }
        StateCommand::Resolve { doc, id } => {
            let resolved = state_docs.resolve_annotation(&doc, &id)?;
            Ok(json!({
                "success": resolved.is_some(),
                "annotation": resolved,
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
    let guard_db_path = project_root.join("guard.db");
    let guard_db = crabjar_guard::GuardDb::open(&guard_db_path)
        .unwrap_or_else(|_| {
            let temp_dir = tempfile::tempdir().unwrap();
            crabjar_guard::GuardDb::open(temp_dir.path().join("guard.db")).unwrap()
        });
    
    let bridge = KnowledgeBridge::new("knowledge.db", project_root, None)?
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
    let guard_db = crabjar_guard::GuardDb::open(project_root.join("guard.db"))
        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(":memory:").expect("guard db fallback"));

    let gate = crabjar_guard::ExecutionGate::new(&guard_db, dry_run, &project_root);

    let gate_result = gate.check(crabjar_guard::GateContext {
        action_type: "exec",
        command,
        args: args.to_vec(),
        trust_layer: 3,
        confidence: crabjar_guard::TrustScore::new(0.9),
        source_event_id: Some(reason),
        can_interrupt: true,
        pid: None,
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
            let flight_db_path =
                crabjar_guard::GuardDb::from_mirror_path(project_root.join("guard.db"));
            let flight_conn = rusqlite::Connection::open(&flight_db_path)?;
            let mut flight_recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
                &flight_conn,
                "exec-session",
            );
            flight_recorder.init()?;

            let cmd_id = flight_recorder
                .execute_command(command, args, &effective_cwd, reason)
                .await?;

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);
            let receipt = records.first().map(|r| r.receipt.clone()).unwrap_or_default();

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
) -> Result<i32, dinit::DinitError> {
    let effective_socket = match socket_path {
        Some(s) => s.clone(),
        None => dinit::default_socket_path(),
    };

    // Build dinitctl args with crabjar-dinit socket
    let mut dinit_args: Vec<String> = vec![
        "-p".to_string(),
        effective_socket.clone(),
    ];
    dinit_args.extend(args.iter().cloned());
    let dinit_args: Vec<&str> = dinit_args.iter().map(|s| s.as_str()).collect();

    let output = std::process::Command::new("dinitctl")
        .args(&dinit_args)
        .current_dir(cwd)
        .output()
        .map_err(|e| dinit::DinitError::DinitctlError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(dinit::DinitError::DinitctlError(
            stderr.trim().to_string(),
        ));
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
    let guard_db = crabjar_guard::GuardDb::open(project_root.join("guard.db"))
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
    }
}

/// Handle bitwarden commands
fn handle_bitwarden_command(
    command: BitwardenCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        BitwardenCommand::Status => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "status": "not_available",
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let status = bitwarden::cli::status()?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "status": "available",
                    "session": status,
                },
            }))
        }
        BitwardenCommand::List { folder, collection } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "items": [],
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let items = bitwarden::cli::list_items(folder.as_deref(), collection.as_deref())?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "items": items,
                },
            }))
        }
        BitwardenCommand::Get { id } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "item": null,
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let item = bitwarden::cli::get_item(&id)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "item": item,
                },
            }))
        }
        BitwardenCommand::Search { query } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "items": [],
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let items = bitwarden::cli::search_items(&query)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "items": items,
                    "query": query,
                },
            }))
        }
        BitwardenCommand::Generate {
            length,
            uppercase,
            lowercase,
            numbers,
            special,
        } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "password": null,
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let password =
                bitwarden::cli::generate_password(length, uppercase, lowercase, numbers, special)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "password": password,
                },
            }))
        }
    }
}

/// Doctor status for a single check
fn doctor_status(name: &str, ok: bool, detail: impl Into<String>) -> serde_json::Value {
    json!({
        "check": name,
        "ok": ok,
        "detail": detail.into(),
    })
}

/// Handle doctor commands
async fn handle_doctor_command(
    command: DoctorCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let mut checks = Vec::new();

    match command {
        DoctorCommand::Check => {
            // 1. Guard DB check
            let guard_path = project_root.join("guard.db");
            let guard_exists = guard_path.exists();
            let guard_ok = guard_exists;
            let guard_detail = if guard_exists {
                format!("exists at {}", guard_path.display())
            } else {
                "not found".to_string()
            };
            checks.push(doctor_status("guard_db", guard_ok, &guard_detail));

            if guard_exists {
                match crabjar_guard::GuardDb::open(&guard_path) {
                    Ok(guard_db) => {
                        let conn = guard_db.conn();
                        let table_count: i64 = conn
                            .query_row(
                                "SELECT count(*) FROM sqlite_master WHERE type='table'",
                                [],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let schema_ok = table_count >= 4;
                        let schema_detail = if schema_ok {
                            format!("schema intact ({} tables)", table_count)
                        } else {
                            format!("schema degraded ({} tables, expected >= 4)", table_count)
                        };
                        checks.push(doctor_status("guard_schema", schema_ok, &schema_detail));

                        let pending_count: i64 = conn
                            .query_row("SELECT count(*) FROM pending_queue WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_pending_queue",
                            true,
                            format!("{} pending entries", pending_count),
                        ));

                        let interrupted_count: i64 = conn
                            .query_row("SELECT count(*) FROM interrupted_log WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_interrupted_log",
                            true,
                            format!("{} interrupted entries", interrupted_count),
                        ));

                        let outcomes_count: i64 = conn
                            .query_row("SELECT count(*) FROM action_outcomes WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_action_outcomes",
                            true,
                            format!("{} recorded outcomes", outcomes_count),
                        ));

                        let pid_trust_count: i64 = conn
                            .query_row("SELECT count(*) FROM pid_trust WHERE 1=1", [], |r| r.get(0))
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_pid_trust",
                            true,
                            format!("{} PID trust records", pid_trust_count),
                        ));

                        let node_count: i64 = conn
                            .query_row("SELECT count(*) FROM memory_nodes WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_memory_graph",
                            true,
                            format!("{} memory nodes", node_count),
                        ));

                        let action_count: i64 = conn
                            .query_row("SELECT count(*) FROM action_requests WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "guard_action_requests",
                            true,
                            format!("{} action requests", action_count),
                        ));
                    }
                    Err(e) => {
                        checks.push(doctor_status(
                            "guard_schema",
                            false,
                            format!("open failed: {}", e),
                        ));
                    }
                }
            }

            // 2. Telemetry / flight recorder DB check
            let flight_path = project_root.join("flight.db");
            let flight_exists = flight_path.exists();
            let flight_ok = flight_exists;
            let flight_detail = if flight_exists {
                format!("exists at {}", flight_path.display())
            } else {
                "not found (created on first exec)".to_string()
            };
            checks.push(doctor_status(
                "flight_recorder_db",
                flight_ok,
                &flight_detail,
            ));

            if flight_exists {
                match rusqlite::Connection::open(&flight_path) {
                    Ok(conn) => {
                        let table_count: i64 = conn
                            .query_row(
                                "SELECT count(*) FROM sqlite_master WHERE type='table'",
                                [],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let schema_ok = table_count >= 4;
                        let schema_detail = if schema_ok {
                            format!("schema intact ({} tables)", table_count)
                        } else {
                            format!("schema degraded ({} tables, expected >= 4)", table_count)
                        };
                        checks.push(doctor_status("flight_schema", schema_ok, &schema_detail));

                        let record_count: i64 = conn
                            .query_row("SELECT count(*) FROM flight_records WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "flight_records",
                            true,
                            format!("{} records", record_count),
                        ));

                        let checkpoint_count: i64 = conn
                            .query_row(
                                "SELECT count(*) FROM session_checkpoint WHERE 1=1",
                                [],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "flight_checkpoints",
                            true,
                            format!("{} checkpoints", checkpoint_count),
                        ));
                    }
                    Err(e) => {
                        checks.push(doctor_status(
                            "flight_schema",
                            false,
                            format!("open failed: {}", e),
                        ));
                    }
                }
            }

            // 3. Knowledge store (memory) DB check
            let memory_path = project_root.join("knowledge.db");
            let memory_exists = memory_path.exists();
            let memory_ok = memory_exists;
            let memory_detail = if memory_exists {
                format!("exists at {}", memory_path.display())
            } else {
                "not found (created on first knowledge operation)".to_string()
            };
            checks.push(doctor_status("knowledge_db", memory_ok, &memory_detail));

            if memory_exists {
                match rusqlite::Connection::open(&memory_path) {
                    Ok(conn) => {
                        let table_count: i64 = conn
                            .query_row(
                                "SELECT count(*) FROM sqlite_master WHERE type='table'",
                                [],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        let schema_ok = table_count >= 3;
                        let schema_detail = if schema_ok {
                            format!("schema intact ({} tables)", table_count)
                        } else {
                            format!("schema degraded ({} tables, expected >= 3)", table_count)
                        };
                        checks.push(doctor_status("knowledge_schema", schema_ok, &schema_detail));

                        let entry_count: i64 = conn
                            .query_row(
                                "SELECT count(*) FROM knowledge_entries WHERE 1=1",
                                [],
                                |r| r.get(0),
                            )
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "knowledge_entries",
                            true,
                            format!("{} entries", entry_count),
                        ));

                        let event_count: i64 = conn
                            .query_row("SELECT count(*) FROM event_rows WHERE 1=1", [], |r| {
                                r.get(0)
                            })
                            .unwrap_or(0);
                        checks.push(doctor_status(
                            "knowledge_events",
                            true,
                            format!("{} events", event_count),
                        ));
                    }
                    Err(e) => {
                        checks.push(doctor_status(
                            "knowledge_schema",
                            false,
                            format!("open failed: {}", e),
                        ));
                    }
                }
            }

            // 4. Workspace config check
            let project_root_path = project_root.to_string_lossy().to_string();
            let _ = project_root_path; // suppress unused warning
            let mut loader = ProjectLoader::new();
            if loader.load_from_directory(&project_root).await.is_ok() {
                if let Some(cfg) = loader.get_current_config() {
                    checks.push(json!({
                        "check": "workspace_config",
                        "ok": true,
                        "detail": format!(
                            "name={}, tools={}, exec_enabled={}",
                            cfg.name, cfg.tools.len(), cfg.tool_execution_enabled
                        ),
                    }));
                } else {
                    checks.push(json!({
                        "check": "workspace_config",
                        "ok": false,
                        "detail": "loaded but no valid config found",
                    }));
                }
            } else {
                checks.push(json!({
                    "check": "workspace_config",
                    "ok": false,
                    "detail": "load failed",
                }));
            }

            // 5. Dinit integration check
            let dinit_path = which::which("dinitctl").ok();
            let dinit_ok = dinit_path.is_some();
            let dinit_detail = if dinit_ok {
                format!("found at {}", dinit_path.unwrap().display())
            } else {
                "not found".to_string()
            };
            checks.push(doctor_status("tool_dinitctl", dinit_ok, &dinit_detail));

            if let Some(ref cfg) = loader.get_current_config() {
                if let Some(ref socket) = cfg.user_dinit_socket {
                    let socket_exists = std::path::Path::new(socket).exists();
                    checks.push(doctor_status(
                        "dinit_socket",
                        socket_exists,
                        format!("{}: {}", socket, if socket_exists { "exists" } else { "not found" }),
                    ));
                }
            }

            // 6. Tool availability
            let git_path = which::which("git").ok();
            checks.push(doctor_status(
                "tool_git",
                git_path.is_some(),
                git_path
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "not found".to_string()),
            ));

            let cargo_path = which::which("cargo").ok();
            checks.push(doctor_status(
                "tool_cargo",
                cargo_path.is_some(),
                cargo_path
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "not found".to_string()),
            ));

            // 7. Bitwarden availability
            let bw_available = bitwarden::cli::is_available();
            checks.push(doctor_status(
                "tool_bitwarden",
                bw_available,
                if bw_available {
                    "available"
                } else {
                    "not found or not logged in"
                },
            ));

            // 8. State-docs directory check
            let state_docs_path = project_root.join("state-docs");
            let state_docs_exists = state_docs_path.exists();
            checks.push(doctor_status(
                "state_docs_dir",
                state_docs_exists,
                if state_docs_exists {
                    format!("exists at {}", state_docs_path.display())
                } else {
                    "not found".to_string()
                },
            ));

            let all_ok = checks.iter().all(|c| c["ok"].as_bool() == Some(true));

            Ok(json!({
                "success": true,
                "doctor": {
                    "ok": all_ok,
                    "checks": checks,
                    "doubt": {
                        "assumptions": [
                            "guard.db schema tables >= 4 indicates healthy state",
                            "flight.db schema tables >= 4 indicates healthy state",
                            "knowledge.db schema tables >= 3 indicates healthy state",
                        ],
                        "blind_spots": [
                            "Does not verify WAL journal integrity",
                            "Does not verify index health",
                            "Schema table count thresholds are heuristic",
                        ],
                        "last_validation": chrono::Utc::now().to_rfc3339(),
                        "stale_after": chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(24))
                            .unwrap_or_default()
                            .to_rfc3339(),
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
                    unsafe { std::env::set_var("INFERENCE_BACKEND", &backend); }
                    Ok(json!({
                        "success": true,
                        "message": format!("Inference backend set to: {}", backend),
                    }))
                }
                _ => Err(format!("Invalid backend: {}. Use 'lm-studio' or 'native'.", backend)),
            }
        }
        BackendCommand::Get => {
            let current_backend = std::env::var("INFERENCE_BACKEND").unwrap_or_else(|_| "lm-studio".to_string());
            Ok(json!({
                "success": true,
                "current_backend": current_backend,
            }))
        }
    }
}
