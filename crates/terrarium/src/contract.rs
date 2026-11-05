//! The contract — the *consumer* side of the glass.
//!
//! These structs are the terrarium's view of the dagr v3 `run.json` produced
//! by `crabjar habitat contract`. They are deliberately a **projection**: only
//! the fields the terrarium reads are modeled, and every field is `Option`
//! with a `#[serde(default)]` so a partial or evolving contract never fails to
//! parse. The terrarium looks *in* at the contract as data; it never names a
//! crabjar type and never writes the file.
//!
//! Field names match the producer exactly (`crabjar/src/habitat_contract.rs`):
//! `task.state`, `attempt.state`, `outcome.{result,evidence,reason,receipt}`,
//! `needs_user`, `attempts[].n`, `projects[].{id,title}`, `events[].{at,type,detail}`.

use serde::Deserialize;

/// The root of a dagr v3 run contract.
#[derive(Debug, Clone, Deserialize)]
pub struct Run {
    /// Contract schema version (3).
    #[serde(default)]
    pub dagr: u32,
    #[serde(default)]
    pub run: RunMeta,
    /// ISO-8601 generation timestamp.
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunMeta {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
}

/// A unit of work. One crab per task (per-task identity, spec §9.3).
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    /// Project id this task belongs to (its home zone). Absent for run-root
    /// tasks like `THEORY`.
    #[serde(default)]
    pub project: Option<String>,
    /// Raw task state (`queued`/`review`/`blocked`/…). The *effective* state is
    /// derived from this plus the latest attempt — see `simulator::effective_state`.
    #[serde(default)]
    pub state: Option<String>,
    /// When true the task is waiting on a human (guard pending). Forces
    /// `blocked` regardless of anything else (spec §5).
    #[serde(default)]
    pub needs_user: bool,
    #[serde(default)]
    pub unblock: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attempt {
    #[serde(default)]
    pub id: Option<String>,
    /// 1-based rework counter. Drives shell age (spec §9.3).
    #[serde(default)]
    pub n: u32,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    // Exhaust metrics (AI agent token/cost signals)
    #[serde(default)]
    pub tokens_in: Option<u32>,
    #[serde(default)]
    pub tokens_out: Option<u32>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub cost_cents: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Outcome {
    /// `done`/`pass`/`failed`/`fail`/… — the verdict of the attempt.
    #[serde(default)]
    pub result: Option<String>,
    /// Evidence tier: `verified`/`reported`/`heuristic`/`asserted`.
    #[serde(default)]
    pub evidence: Option<String>,
    /// The *why* of a failure (or the receipt of a pass). Never dropped —
    /// surfaced as the crab's thought bubble (spec §5).
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub attempt: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

impl Run {
    /// Parse a contract from a file. Returns `Err` with the path and the parse
    /// error — the caller decides whether that's fatal (it is, for `view`).
    ///
    /// Handles both raw DAGR v3 JSON and crabjar's envelope format
    /// (`{doubt, message, payload: {contract}, success}`).
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;

        // Try parsing as crabjar envelope first (has payload.contract wrapper)
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw)
            && parsed
                .get("payload")
                .and_then(|p| p.get("contract"))
                .is_some()
        {
            // Unwrap envelope — payload.contract is the actual DAGR object
            let contract_val = parsed["payload"]["contract"].clone();
            return serde_json::from_value(contract_val)
                .map_err(|e| format!("parse {} (envelope): {e}", path.display()));
        }

        // Fall back to raw DAGR v3 parsing
        serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// The exact shape the crabjar producer emits (triage task with a failed
    /// attempt + guard task + theory task). Parsing this proves the consumer
    /// and producer agree on the wire.
    const PRODUCER_SHAPE: &str = r#"{"dagr": 3, "run": {"id": "crabjar-habitat", "title": "t", "started_at": "2026-09-01T00:00:00Z"}, "generated_at": "2026-09-01T00:16:00Z", "projects": [{"id": "triage", "title": "attempt triage (ADR-006)", "owner": "maintainer"}, {"id": "guard", "title": "guard pending queue", "owner": "user"}], "tasks": [{"id": "T1", "title": "get the workspace test suite green", "kind": "review", "owner": "maintainer", "project": "triage", "state": "review", "deps": [], "note": "make the test suite pass", "attempts": [{"id": "T1·a1", "n": 1, "cause": {"type": "initial"}, "actor": "agent", "state": "done", "started_at": "2026-09-01T00:00:30Z", "ended_at": "2026-09-01T00:01:00Z", "outcome": {"result": "done", "evidence": "reported", "receipt": "exit 0"}}]}, {"id": "G1", "title": "rm -rf target", "kind": "question", "owner": "user", "project": "guard", "state": "blocked", "deps": [], "unblock": "user", "note": "destructive · trust L2 · conf 0.40", "attempts": []}, {"id": "THEORY", "title": "theory state-doc", "kind": "docs", "owner": "maintainer", "state": "queued", "deps": [], "note": "not yet indexed", "attempts": []}], "events": [{"at": "2026-09-01T00:01:00Z", "type": "attempt_settled", "attempt": "T1·a1", "detail": "settled — exit 0"}, {"at": "2026-09-01T00:16:00Z", "type": "note", "detail": "habitat contract generated"}]}"#;

    #[test]
    fn parses_producer_shape() {
        let run: Run = serde_json::from_str(PRODUCER_SHAPE).expect("producer shape must parse");
        assert_eq!(run.dagr, 3);
        assert_eq!(run.projects.len(), 2);
        assert_eq!(run.tasks.len(), 3);
        assert_eq!(run.events.len(), 2);
    }

    #[test]
    fn project_is_optional_on_theory_task() {
        let run: Run = serde_json::from_str(PRODUCER_SHAPE).unwrap();
        let theory = run.tasks.iter().find(|t| t.id == "THEORY").unwrap();
        assert!(theory.project.is_none());
        let t1 = run.tasks.iter().find(|t| t.id == "T1").unwrap();
        assert_eq!(t1.project.as_deref(), Some("triage"));
    }

    #[test]
    fn needs_user_defaults_false() {
        let run: Run = serde_json::from_str(PRODUCER_SHAPE).unwrap();
        assert!(!run.tasks.iter().any(|t| t.needs_user));
    }

    #[test]
    fn empty_contract_parses() {
        let run: Run = serde_json::from_str(r#"{"dagr":3}"#).unwrap();
        assert!(run.tasks.is_empty());
        assert!(run.projects.is_empty());
        assert!(run.events.is_empty());
    }

    #[test]
    fn missing_outcome_fields_default() {
        let run: Run = serde_json::from_str(PRODUCER_SHAPE).unwrap();
        let g1 = run.tasks.iter().find(|t| t.id == "G1").unwrap();
        assert!(g1.attempts.is_empty());
        let t1 = run.tasks.iter().find(|t| t.id == "T1").unwrap();
        let a = &t1.attempts[0];
        assert_eq!(
            a.outcome.as_ref().unwrap().evidence.as_deref(),
            Some("reported")
        );
        assert_eq!(
            a.outcome.as_ref().unwrap().receipt.as_deref(),
            Some("exit 0")
        );
    }

    #[test]
    fn parses_crabjar_envelope() {
        let envelope = r#"{"doubt": {}, "message": "test", "payload": {"contract": {"dagr": 3, "run": {"id": "env-test"}, "generated_at": null, "projects": [], "tasks": [{"id": "T1", "title": "envelope test", "state": "running"}], "events": []}}, "success": true}"#;

        // Write to temp file and use Run::load() which handles envelope unwrapping
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", envelope).unwrap();
        let run = Run::load(f.path()).expect("envelope must parse");

        assert_eq!(run.dagr, 3);
        assert_eq!(run.tasks.len(), 1);
        assert_eq!(run.tasks[0].id, "T1");
    }
}
