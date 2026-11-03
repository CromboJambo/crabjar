use clap::Parser;
use commands::backend::handle_backend_command;
use commands::dotfile::handle as handle_dotfile_command;
use commands::ha::handle_ha_command;
use commands::habitat::handle as handle_habitat_command;
use commands::metrics::handle_metrics_command;
use commands::memory::handle_memory_command;
use commands::state::handle as handle_state_command;
use commands::fleet::handle_fleet_submit_kanban_task;
use commands::guard::handle as handle_guard_command;
use commands::knowledge::handle_knowledge_command;
use commands::slow_friend::handle as handle_slow_friend_command;
use commands::attempts::handle as handle_attempts_command;
use commands::exec::handle_exec;
use commands::workspace::handle_workspace_status;
use serde_json::json;

mod bitwarden;
mod crabjar_config;
mod doctor;
mod dotfile_manager;
mod hermes_memory;
mod homeassistant;
mod knowledge_store;
mod mcp_client_cli;
mod metrics;
mod project_loader;
mod tool_registry_cli;

use bitwarden::commands::handle_bitwarden_command;
mod commands;
use crabjar_lib::{
    ApprovalCommand, BitwardenCommand, Cli, CliCommand, DoctorCommand, KnowledgeCommand, WorkspaceCommand,
};
use doctor::handle_doctor_command;
use mcp_client_cli::handle_mcp_command;
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
            container,
            otel_endpoint,
            target,
        }) => {
            if let Some(t) = &target {
                // Remote execution via SSH bastion (Phase 1 MVP)
                match crabjar_ssh_bastion::execute_remote(t, &command, &args, None).await {
                    Ok(output) => serde_json::json!({
                        "success": true,
                        "target": t,
                        "output": output.trim()
                    }),
                    Err(err) => {
                        error_response(&format!("remote execution failed on {}: {}", t, err), false)
                    }
                }
            } else {
                // Local execution (existing behavior)
                handle_exec(
                    &command,
                    &args,
                    &cwd,
                    &reason,
                    dry_run,
                    container,
                    &otel_endpoint,
                )
                .await
                .unwrap_or_else(|err| error_response(&err.to_string(), true))
            }
        }
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
        Some(CliCommand::Mcp { command }) => handle_mcp_command(command)
            .await
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Metrics { command }) => handle_metrics_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Habitat { command }) => handle_habitat_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Attempts { command }) => handle_attempts_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Decide {
            subcmd,
            observation,
            criteria_file,
            backend,
            threshold,
            verbose,
            fresh,
            obs_hash,
            outcome,
            last,
            decision_id,
            component,
        }) => match (subcmd.as_str(), observation, criteria_file) {
            ("evaluate", Some(obs), Some(crit)) => {
                match commands::semantic::handle_evaluate(
                    &obs, &crit, &backend, threshold, verbose, fresh,
                ) {
                    Ok(json) => json,
                    Err(err) => error_response(&err.to_string(), true),
                }
            }
            ("evaluate", None, _) => {
                error_response("Missing --observation for decide evaluate", true)
            }
            ("evaluate", _, None) => {
                error_response("Missing --criteria-file for decide evaluate", true)
            }
            ("feedback", _, _) => {
                match commands::semantic::handle_feedback(obs_hash.as_deref(), outcome.as_deref()) {
                    Ok(json) => json,
                    Err(err) => error_response(&err.to_string(), true),
                }
            }
            ("metrics", _, _) => match commands::semantic::handle_metrics(&last) {
                Ok(json) => json,
                Err(err) => error_response(&err.to_string(), true),
            },
            ("promote-rule", _, _) => match commands::semantic::handle_promote_rule(decision_id) {
                Ok(json) => json,
                Err(err) => error_response(&err.to_string(), true),
            },
            ("learnings", _, _) => {
                match commands::semantic::handle_learnings(component.as_deref()) {
                    Ok(json) => json,
                    Err(err) => error_response(&err.to_string(), true),
                }
            }
            ("list-criteria", _, search_path) => {
                match commands::semantic::handle_list_criteria(search_path.as_deref()) {
                    Ok(json) => json,
                    Err(err) => error_response(&err.to_string(), true),
                }
            }
            ("validate", _, Some(file)) => match commands::semantic::handle_validate(&file) {
                Ok(json) => json,
                Err(err) => error_response(&err.to_string(), true),
            },
            ("validate", _, None) => {
                error_response("Missing --criteria-file for decide validate", true)
            }
            _ => error_response(
                "Unknown decide subcommand. Use: evaluate, feedback, metrics, promote-rule, learnings, list-criteria, validate",
                true,
            ),
        },
        Some(CliCommand::SlowFriend { command }) => handle_slow_friend_command(command)
            .unwrap_or_else(|err| error_response(&err.to_string(), true)),
        Some(CliCommand::Memory { command }) => handle_memory_command(command),
        Some(CliCommand::Ha { command }) => handle_ha_command(command),
        Some(CliCommand::Learn {
            subcmd,
            min_invocations,
        }) => {
            let analyzer = crabjar_lib::learning_loop::LearningAnalyzer::new();
            match subcmd.as_str() {
                "analyze" => match analyzer.generate_recommendations() {
                    Ok(recommendations) => json!({
                        "success": true,
                        "type": "learn_analysis",
                        "recommendations": recommendations
                    }),
                    Err(err) => error_response(&err.to_string(), true),
                },
                "training-candidates" => match analyzer.export_training_candidates(min_invocations)
                {
                    Ok(candidates) => json!({
                        "success": true,
                        "type": "training_candidates",
                        "candidates": candidates,
                        "count": candidates.len()
                    }),
                    Err(err) => error_response(&err.to_string(), true),
                },
                "failure-patterns" => match analyzer.identify_failure_patterns() {
                    Ok(patterns) => json!({
                        "success": true,
                        "type": "failure_patterns",
                        "patterns": patterns,
                        "count": patterns.len()
                    }),
                    Err(err) => error_response(&err.to_string(), true),
                },
                _ => error_response(
                    "Unknown learn subcommand. Use: analyze, training-candidates, failure-patterns",
                    true,
                ),
            }
        }
        Some(CliCommand::Session { command }) => {
            let mut search = crabjar_lib::session_search::SessionSearchIndex::new();
            match command {
                crabjar_lib::SessionCommand::Search { query, limit } => {
                    match search.search(&query, limit) {
                        Ok(results) => json!({
                            "success": true,
                            "type": "session_search",
                            "query": query,
                            "results": results,
                            "count": results.len()
                        }),
                        Err(err) => error_response(&err.to_string(), true),
                    }
                }
                crabjar_lib::SessionCommand::Related { topic } => {
                    match search.find_related(&topic) {
                        Ok(results) => json!({
                            "success": true,
                            "type": "session_related",
                            "topic": topic,
                            "results": results,
                            "count": results.len()
                        }),
                        Err(err) => error_response(&err.to_string(), true),
                    }
                }
                crabjar_lib::SessionCommand::Failures { error_type } => {
                    match search.find_past_failures(&error_type) {
                        Ok(results) => json!({
                            "success": true,
                            "type": "session_failures",
                            "error_type": error_type,
                            "results": results,
                            "count": results.len()
                        }),
                        Err(err) => error_response(&err.to_string(), true),
                    }
                }
            }
        }
        Some(CliCommand::Fleet { command }) => match command {
            crabjar_lib::FleetCommand::SubmitKanbanTask {
                task_id,
                conductor_url,
                token,
            } => handle_fleet_submit_kanban_task(&task_id, &conductor_url, &token).await,
        },
        Some(CliCommand::Checkpoint { command }) => {
            commands::checkpoint::handle_checkpoint_command(command)
                .unwrap_or_else(|err| error_response(&err.to_string(), true))
        }
        Some(CliCommand::Approval { command }) => commands::approval::handle_approval_command(command),
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

// handle_knowledge_command moved to src/commands/knowledge.rs

// handle_workspace_status moved to src/commands/workspace.rs

// handle_fleet_submit_kanban_task and submit_goal_to_conductor moved to src/commands/fleet.rs and src/commands/goals.rs
// handle_exec moved to src/commands/exec.rs

// execute_via_dinitctl moved to src/commands/dinitctl.rs
// execute_in_podman_container moved to src/commands/podman.rs

// handle_guard_command moved to src/commands/guard.rs

// handle_attempts_command moved to src/commands/attempts.rs

// handle_slow_friend_command moved to src/commands/slow_friend.rs

// handle_memory_command moved to src/commands/memory.rs

// handle_ha_command moved to src/commands/ha.rs

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
        "crabjar attempts status",
        "crabjar attempts rewind <commit> [--workdir <dir>] [--trunk <sha>] [--dry-run]",
        "crabjar habitat contract [--queue-path <p>] [--guard-db <p>] [--out <p>]",
        "crabjar tool list",
        "crabjar tool search --query=<q> [--stars=<n>] [--sort=stars|updated] [--limit=10]",
        "crabjar tool install --repo=<org/repo> [--target=<path>]",
        "crabjar tool clawhub-search --query=<q> [--limit=10]",
        "crabjar tool clawhub-install --name=<skill> [--version=<ver>] [--target=<path>]",
        "crabjar tool scan <path> [--checksum <sha256>]",
    ]
}

// handle_backend_command moved to src/commands/backend.rs
// handle_metrics_command moved to src/commands/metrics.rs
