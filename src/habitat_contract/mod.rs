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

pub mod generate;
pub mod readers;
pub mod types;

pub use generate::build_contract;
pub use readers::{read_pending_actions, read_theory_status};
pub use types::{PendingAction, TheoryStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kanban_source;
    use chrono::Utc;
    use crabjar_terminal::{Attempt, AttemptStatus, Condition, Receipt, TriageQueue};
    use serde_json::json;

    fn mk_attempt(id: u64, exit: Option<i32>, broken: bool) -> Attempt {
        Attempt {
            id,
            task_type: crabjar_terminal::TaskType::DriftCheck,
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
            inference_metrics: crabjar_terminal::InferenceMetrics::default(),
        }
    }

    fn task_by_id<'a>(doc: &'a serde_json::Value, id: &'a str) -> &'a serde_json::Value {
        doc["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == json!(id))
            .unwrap_or_else(|| panic!("task {id} not found"))
    }

    #[test]
    fn contract_is_never_empty_run() {
        let q = TriageQueue::new(10);
        let doc = build_contract(&q, &[], &TheoryStatus::default(), &[]);
        assert_eq!(doc["dagr"], 3);
        assert_eq!(doc["run"]["id"], "crabjar-habitat");
        assert!(
            !doc["tasks"].as_array().unwrap().is_empty(),
            "E102: tasks required"
        );
        // The theory doc is always present, even when not indexed.
        assert_eq!(task_by_id(&doc, "THEORY")["state"], "queued");
    }

    #[test]
    fn passing_attempt_is_review_task_with_done_attempt() {
        let mut q = TriageQueue::new(10);
        q.attempts.push_back(mk_attempt(7, Some(0), false));
        let doc = build_contract(&q, &[], &TheoryStatus::default(), &[]);
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
        let doc = build_contract(&q, &[], &TheoryStatus::default(), &[]);
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
        let doc = build_contract(&q, &pending, &TheoryStatus::default(), &[]);
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
        let doc = build_contract(&q, &[], &theory, &[]);
        assert_eq!(task_by_id(&doc, "THEORY")["state"], "review");
    }

    #[test]
    fn events_are_ascending() {
        let mut q = TriageQueue::new(10);
        q.attempts.push_back(mk_attempt(1, Some(0), false));
        q.attempts.push_back(mk_attempt(2, Some(0), false));
        let doc = build_contract(&q, &[], &TheoryStatus::default(), &[]);
        let evs = doc["events"].as_array().unwrap();
        for w in evs.windows(2) {
            let (a, b) = (w[0]["at"].as_str().unwrap(), w[1]["at"].as_str().unwrap());
            assert!(a <= b, "events out of order (W207): {a} > {b}");
        }
    }

    #[test]
    fn kanban_tasks_appear_in_contract() {
        let q = TriageQueue::new(10);
        let kanban = vec![kanban_source::KanbanTask {
            id: "t_test".to_string(),
            title: "Test task from kanban".to_string(),
            status: "ready".to_string(),
            assignee: Some("default".to_string()),
            priority: 10,
            created_at: Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            result: None,
            current_run_id: None,
        }];
        let doc = build_contract(&q, &[], &TheoryStatus::default(), &kanban);
        let t = task_by_id(&doc, "KANBAN-t_test");
        assert_eq!(t["state"], "queued");
        assert_eq!(t["kind"], "critical");
        assert_eq!(t["project"], "hermes-kanban");
    }
}
