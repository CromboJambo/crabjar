use clap::Parser;
use crabjar_lib::{Cli, CliCommand, StateCommand, WorkspaceCommand};
use serde_json::json;

mod dotfile_manager;
mod knowledge_store;
mod project_loader;
mod state_docs;
mod bitwarden;

use crabjar_lib::{BitwardenCommand, DotfileCommand, GuardCommand, KnowledgeCommand};
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
    let bridge = KnowledgeBridge::new("knowledge.db", project_root, None)?;
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
                    "name": config.workspace_name,
                    "description": config.description,
                    "declared_tools": config.tools.len(),
                    "tool_execution_enabled": config.tool_execution_enabled,
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
    if let Some(config) = loader.get_current_config()
        && !config.tool_execution_enabled
    {
        return Ok(json!({
            "success": false,
            "exec": {
                "command": command,
                "args": args,
                "reason": reason,
                "gate_result": "denied",
                "reason": "tool_execution_enabled is false in config",
            },
        }));
    }

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
            let flight_recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
                &flight_conn,
                "exec-session",
            );
            flight_recorder.init()?;

            let cmd_id = flight_recorder
                .execute_command(command, args, &effective_cwd, reason)
                .await?;

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);

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

            Ok(json!({
                "success": true,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "cmd_id": cmd_id,
                    "exit_code": exit_code,
                    "gate_result": "proceed",
                    "git_dirty": git_dirty,
                    "git_diff_hash": git_diff,
                    "outcome_id": outcome_id,
                    "flight_recorder": true,
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
        GuardCommand::Grant { pid, trust_layer, auto_grant } => {
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
        BitwardenCommand::Generate { length, uppercase, lowercase, numbers, special } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "password": null,
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let password = bitwarden::cli::generate_password(
                length,
                uppercase,
                lowercase,
                numbers,
                special,
            )?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "password": password,
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
    ]
}
