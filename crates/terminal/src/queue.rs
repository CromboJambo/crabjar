//! Bounded triage queue (ADR-006, the Maintainer's Contract).
//!
//! Producer (agent, keeps mapping) / consumer (user, triage) / finite
//! buffer. When the buffer is full, the agent **stops mapping** — new
//! attempts are refused with a structured [`QueueRefusal`] (queue full, N
//! unjudged, oldest age). If the maintainer is absent, the system halts
//! instead of accumulating garbage: a halted map is itself the signal that
//! maintenance is missing.
//!
//! Promote and discard both require an annotated [`Judgment`]; a judgment
//! without its annotation and doubt block is rejected. Judged attempts
//! leave the queue — the durable record is the git graph + the ADR-005
//! stream, not the queue.
//!
//! On disk: JSONL (header line + one attempt per line), the same discipline
//! as [`crate::session_record::SessionRecord`]. The queue serializes the
//! *view*; it is not a third state space.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::attempts::{Attempt, AttemptStatus, Judgment};

/// On-disk format version for the queue record.
pub const QUEUE_VERSION: u32 = 1;

/// The bounded triage queue.
///
/// `budget` is the count bound: when `len >= budget`, [`TriageQueue::push`]
/// is refused. (An age bound is a later refinement; the count bound is what
/// makes the halt mechanical.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageQueue {
    /// Unjudged attempts, in push order.
    pub attempts: VecDeque<Attempt>,
    /// The queue's count budget.
    pub budget: usize,
}

/// A structured refusal: the queue is full and the agent must stop mapping.
///
/// This is a value, not a panic — the refusal is returned to the caller so
/// the agent halts with the reason in hand (queue full, N unjudged, oldest
/// age; triage to continue).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueRefusal {
    /// Why the push was refused.
    pub reason: String,
    /// Unjudged attempts currently in the queue.
    pub unjudged: usize,
    /// The queue's count budget.
    pub budget: usize,
    /// Age of the oldest unjudged attempt (maintenance debt).
    pub oldest_age: Duration,
}

/// The outcome of a [`TriageQueue::push`].
#[derive(Debug, Clone)]
pub enum PushOutcome {
    /// The attempt landed unjudged; the assigned id.
    Recorded { id: u64 },
    /// The queue is full; the agent stops mapping.
    Refused(QueueRefusal),
}

/// Why a judgment was rejected.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum JudgmentError {
    /// No attempt with that id is in the queue.
    #[error("no unjudged attempt with id {0}")]
    NotFound(u64),
    /// The judgment failed the Maintainer's Contract (missing annotation
    /// or doubt block).
    #[error("{0}")]
    Invalid(String),
}

impl TriageQueue {
    /// An empty queue with the given count budget.
    pub fn new(budget: usize) -> Self {
        Self {
            attempts: VecDeque::new(),
            budget,
        }
    }

    /// The number of unjudged attempts.
    pub fn len(&self) -> usize {
        self.attempts.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.attempts.is_empty()
    }

    /// Whether the queue is at budget (the next push will be refused).
    pub fn is_full(&self) -> bool {
        self.len() >= self.budget
    }

    /// Budget usage as a fraction in `0.0..=1.0` (the dashboard's debt
    /// metric). An empty budget reads as full.
    pub fn budget_usage(&self) -> f64 {
        if self.budget == 0 {
            1.0
        } else {
            self.len() as f64 / self.budget as f64
        }
    }

    /// Age of the oldest unjudged attempt (the dashboard's oldest-age).
    pub fn oldest_age(&self) -> Duration {
        let now = Utc::now();
        self.attempts
            .iter()
            .map(|a| (now - a.recorded_at).to_std().unwrap_or(Duration::ZERO))
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// Push an attempt into the queue.
    ///
    /// The attempt lands `Unjudged` with the next monotonic id. When the
    /// queue is at budget, the push is **refused** with a structured
    /// [`QueueRefusal`] — the agent stops mapping; nothing is dropped or
    /// panicked.
    pub fn push(&mut self, mut attempt: Attempt) -> PushOutcome {
        if self.is_full() {
            return PushOutcome::Refused(QueueRefusal {
                reason: "queue full — triage to continue".to_string(),
                unjudged: self.len(),
                budget: self.budget,
                oldest_age: self.oldest_age(),
            });
        }
        attempt.id = self.next_id();
        attempt.status = AttemptStatus::Unjudged;
        self.attempts.push_back(attempt);
        PushOutcome::Recorded {
            id: self.attempts.back().unwrap().id,
        }
    }

    /// The next monotonic id: one past the highest existing id (ids
    /// survive across save/load, so they are never re-minted).
    fn next_id(&self) -> u64 {
        self.attempts
            .iter()
            .map(|a| a.id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0)
    }

    /// Look up an unjudged attempt by id.
    pub fn get(&self, id: u64) -> Option<&Attempt> {
        self.attempts.iter().find(|a| a.id == id)
    }

    /// Judge an attempt (promote or discard).
    ///
    /// The judgment must carry an annotation and a doubt block — the CLI
    /// output contract applied to the user's side. A malformed judgment is
    /// rejected with [`JudgmentError::Invalid`]; the attempt stays
    /// unjudged. A valid judgment removes the attempt from the queue and
    /// returns it (the record lives on in the git graph + stream).
    pub fn judge(&mut self, id: u64, judgment: &Judgment) -> Result<Attempt, JudgmentError> {
        if let Some(err) = judgment.validate() {
            return Err(JudgmentError::Invalid(err));
        }
        let pos = self
            .attempts
            .iter()
            .position(|a| a.id == id)
            .ok_or(JudgmentError::NotFound(id))?;
        let mut attempt = self.attempts.remove(pos).unwrap();
        attempt.status = AttemptStatus::Judged(judgment.clone());
        Ok(attempt)
    }

    /// The unjudged attempts, in push order.
    pub fn unjudged(&self) -> impl Iterator<Item = &Attempt> {
        self.attempts.iter()
    }

    /// Broken conditions across all unjudged attempts, as (attempt id,
    /// condition name) pairs — the dashboard's "awaiting diagnosis."
    pub fn broken_conditions(&self) -> Vec<(u64, String)> {
        self.attempts
            .iter()
            .flat_map(|a| {
                a.broken_conditions()
                    .into_iter()
                    .map(move |c| (a.id, c.name.clone()))
            })
            .collect()
    }

    /// A structured status summary for the dashboard.
    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "unjudged": self.len(),
            "budget": self.budget,
            "budget_usage": self.budget_usage(),
            "full": self.is_full(),
            "oldest_age_secs": self.oldest_age().as_secs(),
            "broken_conditions": self
                .broken_conditions()
                .into_iter()
                .map(|(id, name)| {
                    serde_json::json!({ "attempt_id": id, "condition": name })
                })
                .collect::<Vec<_>>(),
        })
    }

    /// Serialize to JSONL (header line + one attempt per line).
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        let header = serde_json::json!({
            "version": QUEUE_VERSION,
            "budget": self.budget,
        });
        out.push_str(&header.to_string());
        out.push('\n');
        for attempt in &self.attempts {
            out.push_str(&serde_json::to_string(attempt).expect("Attempt serialization cannot fail"));
            out.push('\n');
        }
        out
    }

    /// Parse JSONL (header line + one attempt per line).
    pub fn from_jsonl(text: &str) -> anyhow::Result<Self> {
        let mut lines = text.lines();
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("empty queue record"))?;
        let header: serde_json::Value = serde_json::from_str(header_line)?;
        let version = header
            .get("version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("queue record header missing version"))?;
        if version != u64::from(QUEUE_VERSION) {
            return Err(anyhow::anyhow!(
                "unsupported queue record version {version} (want {QUEUE_VERSION})"
            ));
        }
        let budget = header
            .get("budget")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("queue record header missing budget"))? as usize;
        let attempts = lines
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<Attempt>(line)
                    .map_err(|e| anyhow::anyhow!("bad attempt line: {e}"))
            })
            .collect::<anyhow::Result<VecDeque<_>>>()?;
        Ok(Self { attempts, budget })
    }

    /// Write the queue to `path` as JSONL.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_jsonl())?;
        Ok(())
    }

    /// Load the queue from `path` (JSONL).
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Self::from_jsonl(&fs::read_to_string(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attempts::{Condition, Doubt};
    use crate::stream::Receipt;
    use chrono::DateTime;

    fn receipt(cmd: &str) -> Receipt {
        Receipt {
            command: cmd.to_string(),
            output: String::new(),
            exit_code: Some(1),
            duration: Duration::from_millis(10),
            cwd: None,
        }
    }

    fn attempt(cmd: &str) -> Attempt {
        Attempt {
            id: 0,
            receipt: receipt(cmd),
            parent: "abc1234".to_string(),
            diff: "diff".to_string(),
            preconditions: vec![Condition {
                name: "lockfile".to_string(),
                expected: "pinned".to_string(),
                actual: Some("drifted".to_string()),
                broken: true,
            }],
            invertible: true,
            intent: "trying to hit the wall".to_string(),
            recorded_at: Utc::now(),
            approach_warning: None,
            status: AttemptStatus::Unjudged,
        }
    }

    fn judgment() -> Judgment {
        Judgment {
            annotation: "discard: wrong direction".to_string(),
            doubt: Doubt {
                assumptions: vec!["workdir is a git repo".to_string()],
                blind_spots: vec!["network not observable".to_string()],
                last_validation: "receipt exit code".to_string(),
                stale_after: "next trunk commit".to_string(),
            },
        }
    }

    #[test]
    fn push_lands_unjudged_with_monotonic_ids() {
        let mut q = TriageQueue::new(3);
        let PushOutcome::Recorded { id: i1 } = q.push(attempt("a")) else {
            panic!("expected recorded")
        };
        let PushOutcome::Recorded { id: i2 } = q.push(attempt("b")) else {
            panic!("expected recorded")
        };
        assert_eq!(i1, 0);
        assert_eq!(i2, 1);
        assert_eq!(q.len(), 2);
        assert!(matches!(
            q.get(i1).unwrap().status,
            AttemptStatus::Unjudged
        ));
    }

    #[test]
    fn full_queue_refuses_with_structured_refusal() {
        let mut q = TriageQueue::new(2);
        q.push(attempt("a"));
        q.push(attempt("b"));
        match q.push(attempt("c")) {
            PushOutcome::Refused(refusal) => {
                assert_eq!(refusal.unjudged, 2);
                assert_eq!(refusal.budget, 2);
                assert!(refusal.reason.contains("queue full"));
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        // The refused attempt did not land.
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn judge_requires_annotation_and_doubt() {
        let mut q = TriageQueue::new(2);
        q.push(attempt("a"));
        let id = q.get(0).unwrap().id;

        // Missing annotation.
        let bad = Judgment {
            annotation: "".to_string(),
            doubt: judgment().doubt.clone(),
        };
        assert!(matches!(q.judge(id, &bad), Err(JudgmentError::Invalid(_))));

        // Missing doubt block.
        let bad = Judgment {
            annotation: "promote".to_string(),
            doubt: Doubt::default(),
        };
        assert!(matches!(q.judge(id, &bad), Err(JudgmentError::Invalid(_))));

        // Unknown id.
        assert!(matches!(
            q.judge(99, &judgment()),
            Err(JudgmentError::NotFound(99))
        ));

        // The attempt is still unjudged after the rejections.
        assert_eq!(q.len(), 1);

        // A valid judgment removes it.
        let judged = q.judge(id, &judgment()).unwrap();
        assert!(matches!(judged.status, AttemptStatus::Judged(_)));
        assert!(q.is_empty());
    }

    #[test]
    fn oldest_age_tracks_oldest_attempt() {
        let mut q = TriageQueue::new(3);
        q.push(attempt("a"));
        let mut old = attempt("b");
        old.recorded_at = DateTime::parse_from_rfc3339("2026-08-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        q.push(old);
        assert!(q.oldest_age() > Duration::from_secs(86_400 * 20));
    }

    #[test]
    fn broken_conditions_report_across_queue() {
        let mut q = TriageQueue::new(3);
        q.push(attempt("a"));
        let broken = q.broken_conditions();
        assert_eq!(broken, vec![(0, "lockfile".to_string())]);
    }

    #[test]
    fn queue_jsonl_round_trip_preserves_ids_and_budget() {
        let mut q = TriageQueue::new(3);
        q.push(attempt("a"));
        q.push(attempt("b"));
        let jsonl = q.to_jsonl();
        let mut back = TriageQueue::from_jsonl(&jsonl).unwrap();
        assert_eq!(back.budget, 3);
        assert_eq!(back.len(), 2);
        // Ids survive the round trip (no re-mint).
        assert_eq!(back.get(0).unwrap().receipt.command, "a");
        assert_eq!(back.get(1).unwrap().receipt.command, "b");
        // A push after load continues the id sequence.
        let PushOutcome::Recorded { id } = back.push(attempt("c")) else {
            panic!("expected recorded")
        };
        assert_eq!(id, 2);
    }

    #[test]
    fn load_then_full_still_refuses() {
        let mut q = TriageQueue::new(2);
        q.push(attempt("a"));
        q.push(attempt("b"));
        let mut back = TriageQueue::from_jsonl(&q.to_jsonl()).unwrap();
        assert!(back.is_full());
        assert!(matches!(back.push(attempt("c")), PushOutcome::Refused(_)));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("queue.jsonl");
        let mut q = TriageQueue::new(5);
        q.push(attempt("a"));
        q.save(&path).unwrap();
        let back = TriageQueue::load(&path).unwrap();
        assert_eq!(back.budget, 5);
        assert_eq!(back.len(), 1);
    }
}
