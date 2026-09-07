//! Guard CLI commands (execution gate, trust layers)

use crabjar_lib::GuardCommand;

fn open_guard_db() -> crabjar_guard::GuardDb {
    let project_root = std::env::current_dir().unwrap_or_default();
    crabjar_guard::GuardDb::open(project_root.join("guard/guard.db"))
        .unwrap_or_else(|_| crabjar_guard::GuardDb::open(":memory:").expect("guard db fallback"))
}

pub fn handle(command: GuardCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let guard_db = open_guard_db();

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
            guard_db.update_action_status(&action_id, crabjar_guard::ActionStatus::TrustApproved)?;
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
        GuardCommand::Resolution { limit, effective_layer } => {
            let entries = guard_db.list_trust_resolutions(effective_layer, limit)?;
            let items: Vec<serde_json::Value> = entries.iter().map(|e| {
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
            }).collect();
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
