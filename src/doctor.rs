/// Doctor subcommand: pre-flight system validation.
///
/// Extracted from main.rs to reduce bloat (was ~300 lines of inline SQL).

use serde_json::json;

use crate::DoctorCommand;
use crate::bitwarden;
use crate::ProjectLoader;

/// Doctor status for a single check
pub fn doctor_status(name: &str, ok: bool, detail: impl Into<String>) -> serde_json::Value {
    json!({
        "check": name,
        "ok": ok,
        "detail": detail.into(),
    })
}

/// Handle doctor commands
pub async fn handle_doctor_command(
    command: DoctorCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let mut checks = Vec::new();

    match command {
        DoctorCommand::Check => {
            // 1. Guard DB check
            let guard_path = project_root.join("guard/guard.db");
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
            let flight_path = project_root.join("telemetry/flight.db");
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
            let memory_path = project_root.join("memory/knowledge.db");
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
                            "guard/guard.db schema tables >= 4 indicates healthy state",
                            "telemetry/flight.db schema tables >= 4 indicates healthy state",
                            "memory/knowledge.db schema tables >= 3 indicates healthy state",
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
