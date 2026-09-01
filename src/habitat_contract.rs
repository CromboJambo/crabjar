//! Habitat run-state contract producer (dagr v3).
//!
//! The *producer side* of the habitat dashboard. A pure projection of
//! crabjar's real state into a dagr v3 `run.json`:
//!
//! - the ADR-006 triage queue (unjudged attempts) → project `triage`
//! - the guard pending queue (actions awaiting authorization) → project `guard`
//! - the theory state-doc staleness → run-root `docs` task
//!
//! The renderer (the dagr pane, or a future skin — an isometric terrarium,
//! whatever the user's taste wants) is a **pure function of the emitted
//! file**. This module never renders; it only asserts facts crabjar actually
//! has. If a fact isn't in a source, it is not in the contract (the same
//! "generic over wire representation" stance as the glass: the data surface
//! is the contract, the pixels are disposable).
//!
//! Evidence tiers are honest: a terminal receipt is the *agent's own* report,
//! not independent verification, so it maps to `reported`. Only a gate that
//! ran and passed would earn `verified`.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use crabjar_terminal::TriageQueue;

/// A pending guard action, projected from the `pending_queue` table.
/// Intermediate form so [`build_contract`] stays pure (no SQLite).
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub id: String,
    pub action_type: String,
    pub command: String,
    pub trust_layer: u32,
    pub confidence: f64,
    pub queued_at: i64,
    pub reason: String,
}

/// The theory state-doc's staleness, projected from the state-docs querier.
#[derive(Debug, Clone)]
pub struct TheoryStatus {
    /// Whether the doc is indexed at all (false → not created yet).
    pub indexed: bool,
    /// `fresh | stale | expired | moldy`.
    pub status: String,
    pub days_old: f64,
    pub is_trustworthy: bool,
    pub warning: Option<String>,
}

impl Default for TheoryStatus {
    fn default() -> Self {
        Self {
            indexed: false,
            status: String::new(),
            days_old: 0.0,
            is_trustworthy: false,
            warning: None,
        }
    }
}

/// A contract timestamp: whole seconds, UTC, `Z`-suffixed (dagr E180 wants
/// offsets normalized to UTC).
fn ts(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Truncate a title so the DAG stays readable (the pane is a fixed width).
fn short_title(primary: &str, fallback: &str) -> String {
    let s = if primary.trim().is_empty() { fallback } else { primary };
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
fn theory_attempt(state: &str, evidence: &str, receipt: Option<&str>, reason: Option<&str>) -> Value {
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

/// Build the dagr v3 contract from the three real sources.
///
/// Pure: same inputs → same document (modulo `generated_at` and the theory
/// attempt's `now`-anchored timestamps). Always emits at least one task (the
/// theory doc), so the contract is never an empty run (dagr E102).
pub fn build_contract(
    queue: &TriageQueue,
    pending: &[PendingAction],
    theory: &TheoryStatus,
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

        tasks.push(json!({
            "id": task_id,
            "title": short_title(&a.intent, &a.receipt.command),
            "kind": "review",
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
                vec![theory_attempt("done", "verified", Some("indexed · fresh"), None)],
                format!("fresh · {:.0}d", theory.days_old),
            ),
            "stale" => (
                "review",
                vec![theory_attempt("done", "reported", None, theory.warning.as_deref())],
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

/// Read the guard pending queue. Read-only: opens the DB directly (no schema
/// init) and returns empty if the file or table is absent.
pub fn read_pending_actions(guard_db: &str) -> Vec<PendingAction> {
    let Ok(conn) = rusqlite::Connection::open(guard_db) else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT id, action_type, command, trust_layer, confidence, queued_at, reason \
         FROM pending_queue ORDER BY queued_at DESC",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok(PendingAction {
            id: row.get(0)?,
            action_type: row.get(1)?,
            command: row.get(2)?,
            trust_layer: row.get(3)?,
            confidence: row.get(4)?,
            queued_at: row.get(5)?,
            reason: row.get(6)?,
        })
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

/// Read the theory state-doc's staleness. Returns the not-indexed default
/// when the DB, the migration, or the doc itself is absent.
pub fn read_theory_status(db_path: &str, theory: &str) -> TheoryStatus {
    let Ok(conn) = rusqlite::Connection::open(db_path) else {
        return TheoryStatus::default();
    };
    let Ok(()) = agent_context::state_docs::migrate(&conn) else {
        return TheoryStatus::default();
    };
    let querier =
        agent_context::state_docs::StateDocQuerier::new(conn, std::path::PathBuf::from("state-docs"));
    let status = querier.staleness_status(theory);
    let last_modified = status["last_modified"].as_str().unwrap_or("").to_string();
    if last_modified.is_empty() {
        return TheoryStatus::default();
    }
    TheoryStatus {
        indexed: true,
        status: status["status"].as_str().unwrap_or("fresh").to_string(),
        days_old: status["days_old"].as_f64().unwrap_or(0.0),
        is_trustworthy: status["is_trustworthy"].as_bool().unwrap_or(false),
        warning: status["warning"].as_str().map(String::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabjar_terminal::{Attempt, AttemptStatus, Condition, Receipt};

    fn mk_attempt(id: u64, exit: Option<i32>, broken: bool) -> Attempt {
        Attempt {
            id,
            receipt: Receipt {
                command: "cargo test".to_string(),
                output: String::new(),
                exit_code: exit,
                duration: std::time::Duration::from_secs(30),
                cwd: None,
            },
            parent: "abc123".to_string(),
            diff: String::new(),
            preconditions: if broken {
                vec![Condition {
                    name: "tests-green".to_string(),
                    expected: "pass".to_string(),
                    actual: Some("fail".to_string()),
                    broken: true,
                }]
            } else {
                Vec::new()
            },
            invertible: true,
            intent: "make the test suite pass".to_string(),
            recorded_at: Utc::now(),
            approach_warning: None,
            status: AttemptStatus::Unjudged,
        }
    }

    fn task_by_id<'a>(doc: &'a Value, id: &'a str) -> &'a Value {
        doc["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == id)
            .unwrap_or_else(|| panic!("task {id} not found"))
    }

    #[test]
    fn contract_is_never_empty_run() {
        let q = TriageQueue::new(10);
        let doc = build_contract(&q, &[], &TheoryStatus::default());
        assert_eq!(doc["dagr"], 3);
        assert_eq!(doc["run"]["id"], "crabjar-habitat");
        assert!(!doc["tasks"].as_array().unwrap().is_empty(), "E102: tasks required");
        // The theory doc is always present, even when not indexed.
        assert_eq!(task_by_id(&doc, "THEORY")["state"], "queued");
    }

    #[test]
    fn passing_attempt_is_review_task_with_done_attempt() {
        let mut q = TriageQueue::new(10);
        q.attempts.push_back(mk_attempt(7, Some(0), false));
        let doc = build_contract(&q, &[], &TheoryStatus::default());
        let t = task_by_id(&doc, "T7");
        assert_eq!(t["state"], "review");
        assert_eq!(t["kind"], "review");
        let a = &t["attempts"].as_array().unwrap()[0];
        assert_eq!(a["state"], "done");
        assert_eq!(a["outcome"]["evidence"], "reported");
    }

    #[test]
    fn broken_precondition_marks_attempt_failed_with_reason() {
        let mut q = TriageQueue::new(10);
        q.attempts.push_back(mk_attempt(9, Some(1), true));
        let doc = build_contract(&q, &[], &TheoryStatus::default());
        let t = task_by_id(&doc, "T9");
        let a = &t["attempts"].as_array().unwrap()[0];
        assert_eq!(a["state"], "failed");
        assert_eq!(a["outcome"]["result"], "failed");
        let reason = a["outcome"]["reason"].as_str().unwrap();
        assert!(reason.contains("exit 1"), "reason: {reason}");
        assert!(reason.contains("tests-green"), "reason: {reason}");
    }

    #[test]
    fn guard_pending_is_blocked_question_needing_user() {
        let q = TriageQueue::new(10);
        let pending = vec![PendingAction {
            id: "a1b2c3d4".to_string(),
            action_type: "exec".to_string(),
            command: "rm -rf /tmp/x".to_string(),
            trust_layer: 2,
            confidence: 0.4,
            queued_at: 0,
            reason: "high risk".to_string(),
        }];
        let doc = build_contract(&q, &pending, &TheoryStatus::default());
        let t = task_by_id(&doc, "Ga1b2c3d4");
        assert_eq!(t["state"], "blocked");
        assert_eq!(t["kind"], "question");
        assert_eq!(t["unblock"], "user");
        assert!(t["attempts"].as_array().unwrap().is_empty());
    }

    #[test]
    fn theory_stale_maps_to_review() {
        let q = TriageQueue::new(10);
        let theory = TheoryStatus {
            indexed: true,
            status: "stale".to_string(),
            days_old: 9.0,
            is_trustworthy: true,
            warning: Some("may have drifted".to_string()),
        };
        let doc = build_contract(&q, &[], &theory);
        assert_eq!(task_by_id(&doc, "THEORY")["state"], "review");
    }

    #[test]
    fn events_are_ascending() {
        let mut q = TriageQueue::new(10);
        q.attempts.push_back(mk_attempt(1, Some(0), false));
        q.attempts.push_back(mk_attempt(2, Some(0), false));
        let doc = build_contract(&q, &[], &TheoryStatus::default());
        let evs = doc["events"].as_array().unwrap();
        for w in evs.windows(2) {
            let (a, b) = (w[0]["at"].as_str().unwrap(), w[1]["at"].as_str().unwrap());
            assert!(a <= b, "events out of order (W207): {a} > {b}");
        }
    }
}
