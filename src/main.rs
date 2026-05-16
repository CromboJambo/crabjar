use clap::Parser;
use crabjar_lib::{Cli, CliCommand, StateCommand, WorkspaceCommand};
use serde_json::json;

mod dotfile_manager;
mod knowledge_store;
mod project_loader;
mod state_docs;

use crabjar_lib::{DotfileCommand, KnowledgeCommand};
use dotfile_manager::DotfileManager;
use knowledge_store::KnowledgeBridge;
use knowledge_store::commands::KnowledgeCommandExt;
use project_loader::ProjectLoader;
use state_docs::{AnnotationKind, StateDocsManager};

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
        Some(CliCommand::Exec {
            command,
            args,
            cwd,
            reason,
            dry_run,
        }) => handle_exec(&command, &args, &cwd, &reason, dry_run)
            .await
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

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.get(1).map(String::as_str),
        Some("help" | "--help" | "-h")
    )
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
                    "tool_execution_enabled": false,
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

    // Guard layer: gate check before execution
    let guard_db = crabjar_guard::GuardDb::open(":memory:")
        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(project_root.join("guard.db")).unwrap());

    let gate = crabjar_guard::ExecutionGate::new(&guard_db, dry_run, &project_root);

    let gate_result = gate.check(crabjar_guard::GateContext {
        action_type: "exec",
        command,
        args: args.to_vec(),
        trust_layer: 2,
        confidence: crabjar_guard::TrustScore::new(0.5),
        source_event_id: None,
        has_raw_data: true,
        has_uncertainty: true,
        can_interrupt: true,
    })?;

    match gate_result {
        crabjar_guard::GateResult::DryRun => Ok(json!({
            "success": true,
            "exec": {
                "dry_run": true,
                "command": command,
                "args": args,
                "cwd": cwd,
                "reason": reason,
                "gate_result": "dry_run",
            },
        })),
        crabjar_guard::GateResult::Interrupted {
            reason: gate_reason,
        } => Ok(json!({
            "success": false,
            "exec": {
                "command": command,
                "args": args,
                "cwd": cwd,
                "reason": reason,
                "gate_result": "interrupted",
                "gate_reason": gate_reason,
            },
        })),
        crabjar_guard::GateResult::Pending => Ok(json!({
            "success": false,
            "exec": {
                "command": command,
                "args": args,
                "cwd": cwd,
                "reason": reason,
                "gate_result": "pending",
                "requires_review": true,
            },
        })),
        crabjar_guard::GateResult::Proceed => {
            // Telemetry layer: flight recorder capture
            let dir = tempfile::tempdir()?;
            let flight_db_path = dir.path().join("flight.db");
            let flight_conn = rusqlite::Connection::open(&flight_db_path)?;
            let flight_recorder = crabjar_telemetry::flight_recorder::FlightRecorder::new(
                &flight_conn,
                "exec-session",
            );
            flight_recorder.init()?;

            let cmd_id = flight_recorder
                .execute_command(command, args, cwd, reason)
                .await?;

            let git_dirty = flight_recorder.capture_git_dirty(cwd).await?;
            let git_diff = flight_recorder.capture_git_diff(cwd).await?;

            let records = flight_recorder.query_records(1)?;
            let exit_code = records.first().map(|r| r.exit_code).unwrap_or(-1);

            Ok(json!({
                "success": true,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": cwd,
                    "reason": reason,
                    "cmd_id": cmd_id,
                    "exit_code": exit_code,
                    "gate_result": "proceed",
                    "git_dirty": git_dirty,
                    "git_diff_hash": git_diff,
                    "flight_recorder": true,
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
    ]
}
