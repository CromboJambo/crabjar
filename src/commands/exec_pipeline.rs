//! Execution pipeline helpers for crabjar exec.

use crabjar_guard::{ArbitrationResult, ConciergeOutcome, JevArbiter};
use serde_json::json;

/// Resolve a Queued concierge outcome through the JEV Arbiter.
pub fn resolve_arbiter(
    pending_action: crabjar_guard::GuardedAction<crabjar_guard::Pending>,
    flight_recorder: &mut crabjar_telemetry::flight_recorder::FlightRecorder,
    command: &str,
    args: &[String],
    effective_cwd: &str,
    reason: &str,
    action_request_id: &str,
) -> std::result::Result<ConciergeOutcome, serde_json::Value> {
    let arbiter = JevArbiter::with_standard_passes();
    let arbitration = arbiter.arbitrate(pending_action);

    match arbitration {
        ArbitrationResult::Approved(approved_action, confidence, _evidence) => {
            eprintln!(
                "DEBUG arbiter: approved action (confidence={:.2})",
                confidence
            );
            flight_recorder.record_guard_decision(
                "exec-session",
                "exec",
                "approved",
                3,
                confidence,
            );
            Ok(ConciergeOutcome::Approved(approved_action))
        }
        ArbitrationResult::Denied(denied_action, confidence, evidence) => {
            let reasons: Vec<String> = evidence.iter().map(|e| e.rationale.clone()).collect();
            eprintln!(
                "DEBUG arbiter: denied action (confidence={:.2}) — {}",
                confidence,
                reasons.join("; ")
            );
            flight_recorder.record_guard_decision("exec-session", "exec", "denied", 3, confidence);
            let _ = denied_action;
            Err(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "gate_result": "denied",
                    "arbiter_denied": true,
                    "confidence": confidence,
                    "evidence": evidence.iter().map(|e| json!({
                        "pass": e.source,
                        "delta": e.delta,
                        "rationale": e.rationale,
                    })).collect::<Vec<_>>(),
                },
            }))
        }
        ArbitrationResult::Escalated(excuse, evidence) => {
            let reasons: Vec<String> = evidence.iter().map(|e| e.rationale.clone()).collect();
            eprintln!(
                "DEBUG arbiter: escalated — {} — {}",
                excuse,
                reasons.join("; ")
            );

            let full_command = if args.is_empty() {
                command.to_string()
            } else {
                format!("{} {}", command, args.join(" "))
            };
            let description = format!(
                "[{}] {} (reason: {})",
                action_request_id, full_command, reason
            );
            super::exec_mirror::mirror_pending_to_celld(action_request_id, &description);

            flight_recorder.record_guard_decision("exec-session", "exec", "pending", 3, 0.5);
            Err(json!({
                "success": false,
                "exec": {
                    "command": command,
                    "args": args,
                    "cwd": effective_cwd,
                    "reason": reason,
                    "gate_result": "pending",
                    "requires_review": true,
                    "arbiter_escalated": true,
                    "excuse": excuse.to_string(),
                    "evidence": evidence.iter().map(|e| json!({
                        "pass": e.source,
                        "delta": e.delta,
                        "rationale": e.rationale,
                    })).collect::<Vec<_>>(),
                },
            }))
        }
    }
}
