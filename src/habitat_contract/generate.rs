//! Contract generation: pure projection of crabjar state into dagr v3 format.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};

use crate::kanban_source;
use crabjar_terminal::TriageQueue;

use super::types::{PendingAction, TheoryStatus};

/// A contract timestamp: whole seconds, UTC, `Z`-suffixed (dagr E180 wants
/// offsets normalized to UTC).
fn ts(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Truncate a title so the DAG stays readable (the pane is a fixed width).
fn short_title(primary: &str, fallback: &str) -> String {
    let s = if primary.trim().is_empty() {
        fallback
    } else {
        primary
    };
    let s = s.trim();
    if s.chars().count() > 48 {
        let t: String = s.chars().take(45).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}

/// A terminal attempt for the theory doc task (avoids W203: a settled attempt
/// needs timestamps; `started == ended` is legal — E181 only forbids ending
/// *before* it starts).
fn theory_attempt(
    state: &str,
    evidence: &str,
    receipt: Option<&str>,
    reason: Option<&str>,
) -> Value {
    let at = ts(&Utc::now());
    let mut outcome = json!({ "result": state, "evidence": evidence });
    if let Some(r) = receipt {
        outcome["receipt"] = json!(r);
    }
    if let Some(r) = reason {
        outcome["reason"] = json!(r);
    }
    json!({
        "id": "THEORY·a1",
        "n": 1,
        "cause": { "type": "initial" },
        "actor": "maintainer",
        "state": state,
        "started_at": at,
        "ended_at": at,
        "outcome": outcome,
    })
}

/// Build the dagr v3 contract from all real sources including Hermes kanban.
///
/// Pure: same inputs → same document (modulo `generated_at`). Always emits at
/// least one task, so the contract is never an empty run (dagr E102).
pub fn build_contract(
    queue: &TriageQueue,
    pending: &[PendingAction],
    theory: &TheoryStatus,
    kanban_tasks: &[kanban_source::KanbanTask],
) -> Value {
    let generated = Utc::now();
    let generated_at = ts(&generated);

    let mut tasks: Vec<Value> = Vec::new();
    let mut events: Vec<Value> = Vec::new();

    // ── Triage queue (ADR-006) → project "triage" ────────────────────────
    // Each unjudged attempt is a `review` task (it is the surface awaiting
    // the maintainer's judgment) with a single terminal attempt (the try
    // itself, which produced a receipt).
    let mut earliest: Option<DateTime<Utc>> = None;
    for a in queue.attempts.iter() {
        let task_id = format!("T{}", a.id);
        let att_id = format!("T{}·a1", a.id);
        let broken = a.broken_conditions();
        let exit = a.receipt.exit_code;
        let failed = !broken.is_empty() || exit.map(|c| c != 0).unwrap_or(false);
        let state = if failed { "failed" } else { "done" };
        let started = a.recorded_at - a.receipt.duration;
        let ended = a.recorded_at;
        earliest = Some(match earliest {
            Some(e) if e < ended => e,
            _ => ended,
        });

        // Task type discrimination (ADR-012: subagent delegation).
        let task_kind = match a.task_type {
            crabjar_terminal::TaskType::DriftCheck => "review",
            crabjar_terminal::TaskType::SubagentTask => "delegated",
        };

        let mut outcome = json!({ "result": state, "evidence": "reported" });
        if failed {
            let names: Vec<String> = broken.iter().map(|c| c.name.clone()).collect();
            let mut reason = match exit {
                Some(c) => format!("exit {c}"),
                None => String::new(),
            };
            if !names.is_empty() {
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&names.join(", "));
            }
            outcome["reason"] = json!(reason);
        } else {
            outcome["receipt"] = json!(format!("exit {}", exit.unwrap_or(0)));
        }

        // Exhaust metrics: real values from the LLM runner (Phase 6).
        // Falls back to duration-based estimates only if not yet reported.
        let im = &a.inference_metrics;
        let tokens_in = im.tokens_in.unwrap_or_else(|| {
            let elapsed_secs = a.receipt.duration.as_secs_f64();
            (elapsed_secs * 10.0) as u32 // rough estimate fallback
        });
        let tokens_out = im.tokens_out.unwrap_or_else(|| {
            let elapsed_secs = a.receipt.duration.as_secs_f64();
            (elapsed_secs * 5.0) as u32
        });
        let model_id_value = match &im.model_id {
            Some(mid) => json!(mid),
            None => json!(null),
        };
        let cost_cents = if let Some(_latency) = im.latency_ms {
            // Real latency captured; local inference is "free"
            0.0
        } else {
            a.receipt.duration.as_secs_f64() * 0.001
        };

        tasks.push(json!({
            "id": task_id,
            "title": short_title(&a.intent, &a.receipt.command),
            "kind": task_kind,
            "owner": "maintainer",
            "project": "triage",
            "state": "review",
            "deps": [],
            "note": a.intent,
            "attempts": [
                {
                    "id": att_id,
                    "n": 1,
                    "cause": { "type": "initial" },
                    "actor": "agent",
                    "state": state,
                    "started_at": ts(&started),
                    "ended_at": ts(&ended),
                    "tokens_in": tokens_in,
                    "tokens_out": tokens_out,
                    "model_id": model_id_value,
                    "cost_cents": cost_cents,
                    "outcome": outcome,
                }
            ],
        }));

        let detail = if failed {
            let names: Vec<String> = broken.iter().map(|c| c.name.clone()).collect();
            format!(
                "failed — {}",
                if names.is_empty() {
                    exit.map(|c| format!("exit {c}")).unwrap_or_default()
                } else {
                    names.join(", ")
                }
            )
        } else {
            format!("settled — exit {}", exit.unwrap_or(0))
        };
        events.push(json!({
            "at": ts(&ended),
            "type": "attempt_settled",
            "attempt": att_id,
            "detail": detail,
        }));
    }

    // ── Guard pending queue → project "guard" ────────────────────────────
    // Each pending action is a `blocked` `question` task: it cannot proceed
    // until the user authorizes it (W205 wants the unblock owner named).
    for p in pending {
        tasks.push(json!({
            "id": format!("G{}", p.id),
            "title": short_title(&p.command, &p.action_type),
            "kind": "question",
            "owner": "user",
            "project": "guard",
            "state": "blocked",
            "deps": [],
            "unblock": "user",
            "note": format!("{} · trust L{} · conf {:.2}", p.action_type, p.trust_layer, p.confidence),
            "attempts": [],
        }));
    }

    // ── Hermes kanban board → project "hermes-kanban" ────────────────────
    // Real multi-agent orchestration activity from ~/.hermes/kanban boards.
    // This is the production task queue that crabjar actually feeds work to.
    let dagr_kanban_tasks = kanban_source::to_dagr_tasks(kanban_tasks);
    tasks.extend(dagr_kanban_tasks);

    // Detect stuck boards: ready tasks with no model configured
    let stuck_boards = kanban_source::detect_stuck_boards(kanban_tasks);
    for board_issue in &stuck_boards {
        events.push(json!({
            "at": generated_at,
            "type": "warning",
            "detail": format!("stuck board: {}", board_issue),
        }));
    }

    // ── Theory state-doc → run-root "docs" task ──────────────────────────
    // Staleness tiers map to task states that stay consistent with the
    // attempt record (dagr E150): fresh→done, stale→review,
    // expired/moldy→failed, not-indexed→queued.
    let (t_state, t_attempts, t_note) = if !theory.indexed {
        (
            "queued",
            Vec::new(),
            "theory state-doc not yet created or indexed".to_string(),
        )
    } else {
        match theory.status.as_str() {
            "fresh" => (
                "done",
                vec![theory_attempt(
                    "done",
                    "verified",
                    Some("indexed · fresh"),
                    None,
                )],
                format!("fresh · {:.0}d", theory.days_old),
            ),
            "stale" => (
                "review",
                vec![theory_attempt(
                    "done",
                    "reported",
                    None,
                    theory.warning.as_deref(),
                )],
                format!("stale · {:.0}d", theory.days_old),
            ),
            _ => (
                "failed",
                vec![theory_attempt(
                    "failed",
                    "verified",
                    None,
                    Some(if theory.status == "moldy" {
                        "moldy — regenerate"
                    } else {
                        "expired — regenerate"
                    }),
                )],
                format!("{} — regenerate", theory.status),
            ),
        }
    };
    tasks.push(json!({
        "id": "THEORY",
        "title": "theory state-doc",
        "kind": "docs",
        "owner": "maintainer",
        "state": t_state,
        "deps": [],
        "note": t_note,
        "attempts": t_attempts,
    }));

    // run.started_at = earliest attempt's recorded time, else now.
    let started_at = earliest
        .map(|dt| ts(&dt))
        .unwrap_or_else(|| generated_at.clone());

    let projects = vec![
        json!({ "id": "triage", "title": "attempt triage (ADR-006)", "owner": "maintainer" }),
        json!({ "id": "guard", "title": "guard pending queue", "owner": "user" }),
        json!({ "id": "hermes-kanban", "title": "Hermes kanban board (production tasks)", "owner": "orchestrator" }),
    ];

    // The generation note is the most recent event; sort ascending (W207).
    events.push(json!({
        "at": generated_at,
        "type": "note",
        "detail": "habitat contract generated",
    }));
    events.sort_by(|a, b| {
        a["at"]
            .as_str()
            .unwrap_or("")
            .cmp(b["at"].as_str().unwrap_or(""))
    });

    json!({
        "dagr": 3,
        "run": {
            "id": "crabjar-habitat",
            "title": "crabjar habitat — attempt triage + guard queue",
            "started_at": started_at,
        },
        "generated_at": generated_at,
        "projects": projects,
        "tasks": tasks,
        "events": events,
    })
}
