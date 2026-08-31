//! Attempt graph as falsification record (ADR-006) — the record-only first cut.
//!
//! An [`Attempt`] is a *view* over the two existing structures, not a third
//! state space: it links an ADR-005 [`Receipt`] (the typed terminal stream's
//! addressable cell) to a git commit (the trunk root the branch was cut
//! from). The git commit graph *is* the attempt graph; merge/rebase/revert
//! is the reconciliation machinery.
//!
//! ## The record outlives the understanding
//!
//! Neither party knows what failure looks like yet. The [`Doubt`] block
//! (assumptions, blind_spots, last_validation, stale_after) makes the
//! record's own limits mechanical: the record tags what it assumed and what
//! it couldn't see, so the future reader knows where to look.
//!
//! ## Report
//!
//! [`report`] is what the agent emits instead of freeform "it failed": the
//! failed attempt's diff reference + the broken preconditions as *pointers
//! into the graph* (addressable), with the receipt's local outcome as the
//! reading and a `doubt` block per the CLI output contract.
//!
//! ## What is NOT here
//!
//! - No auto-rewind (second cut). The `invertible` flag records the tier
//!   decision; executing the revert is git's job, next session.
//! - No `STREAM_VERSION` bump. The stream vocabulary does not change this
//!   cut; approach warnings ride on the attempt as annotations, not as new
//!   `TerminalEvent` variants.
//! - No direction tags. The agent records; the user judges, later, via the
//!   annotated [`Judgment`] (see [`crate::queue`]).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::stream::Receipt;

/// The attempt: `Receipt` + `diff` + `preconditions` + `invertible` +
/// `parent` (trunk commit) + `intent` + judgment state.
///
/// The trunk root (`parent`) is a *precondition* of the attempt: an attempt
/// is mergeable/revertable iff the trunk hasn't moved in the regions the
/// attempt touched since that root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    /// Monotonic id, assigned by the triage queue that recorded it.
    pub id: u64,
    /// The ADR-005 receipt: command, output, exit_code, duration, cwd.
    pub receipt: Receipt,
    /// Trunk commit the branch is rooted at (precondition).
    pub parent: String,
    /// The attempt's delta (git diff text or structured ref).
    pub diff: String,
    /// Preconditions the attempt assumed held.
    pub preconditions: Vec<Condition>,
    /// Fine tier (git revert) vs coarse tier (VM destroy + restore).
    /// Non-invertible attempts refuse fine-tier revert.
    pub invertible: bool,
    /// "I was trying to hit X, I got Y." Recorded, not enforced, in this
    /// cut — the seed of mechanical dodge-detection.
    pub intent: String,
    /// When the attempt was recorded (drives the queue's oldest-age).
    pub recorded_at: DateTime<Utc>,
    /// Co-driver tension: the predicted break, emitted before the full
    /// collision. Annotation form (ADR-006 open point, option b) — no
    /// stream change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approach_warning: Option<ApproachWarning>,
    /// Judgment state: unjudged until the maintainer annotates.
    pub status: AttemptStatus,
}

impl Attempt {
    /// Preconditions that are broken — the *why* of a failed attempt.
    pub fn broken_conditions(&self) -> Vec<&Condition> {
        self.preconditions
            .iter()
            .filter(|c| c.broken)
            .collect()
    }
}

/// A precondition the attempt assumed held: a named, checkable expectation.
///
/// `actual` is `None` when the condition was never observed (a blind spot,
/// not a pass). `broken` is the mechanical flag the report's pointers use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    /// The condition's name (stable, addressable).
    pub name: String,
    /// What the attempt expected.
    pub expected: String,
    /// What was observed, when it was observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// Whether the expectation was falsified.
    pub broken: bool,
}

/// Judgment state of an attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttemptStatus {
    /// Lands here on push; the maintainer's queue holds it until annotated.
    Unjudged,
    /// Promoted or discarded — always with an annotated [`Judgment`].
    Judged(Judgment),
}

/// The maintainer's annotated judgment (promote *or* discard).
///
/// A judgment without its annotation and doubt block is rejected by the
/// queue — the CLI output contract's `doubt` block applies to the user's
/// side too. That is the point of the Maintainer's Contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Judgment {
    /// One-line direction judgment (promote) or reason (discard).
    pub annotation: String,
    /// The doubt block *of the judgment itself*.
    pub doubt: Doubt,
}

impl Judgment {
    /// Validate the Maintainer's Contract: annotation and doubt block are
    /// both required. Returns an error string when the judgment is
    /// malformed (the queue rejects it).
    pub fn validate(&self) -> Option<String> {
        if self.annotation.trim().is_empty() {
            return Some(
                "judgment rejected: annotation is required (one-line direction judgment or reason)"
                    .to_string(),
            );
        }
        if self.doubt.assumptions.is_empty() && self.doubt.blind_spots.is_empty() {
            return Some(
                "judgment rejected: doubt block is required (assumptions and/or blind_spots)"
                    .to_string(),
            );
        }
        None
    }
}

/// The doubt block: the record's own limits, made mechanical.
///
/// Shared by the attempt's report, the judgment, and the CLI dashboard.
/// Field names match the CLI output contract exactly (assumptions,
/// blind_spots, last_validation, stale_after).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Doubt {
    /// What the record assumed without verifying.
    #[serde(default)]
    pub assumptions: Vec<String>,
    /// What the record could not observe (processes, network, external
    /// state).
    #[serde(default)]
    pub blind_spots: Vec<String>,
    /// When/what last validated this record.
    pub last_validation: String,
    /// When the record stops being trustworthy.
    pub stale_after: String,
}

/// Co-driver tension: "the theory predicts a break at X, I'm getting close."
///
/// Emitted *before* the full collision so the user can tense up. The
/// stream (ADR-005) captures the approach; this annotation is the
/// *prediction* of the approach.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApproachWarning {
    /// The predicted break ("the theory predicts a break at X").
    pub predicted_break: String,
    /// How close the attempt is to the predicted break (free-form:
    /// "next step", "same command family", ...).
    pub proximity: String,
}

/// The structured report: what the agent emits instead of freeform
/// "it failed."
///
/// The failed attempt's diff reference + the broken preconditions as
/// *pointers into the graph* (addressable — the consumer can jump to the
/// attempt and its conditions), the receipt's local outcome as the reading,
/// prose summary on top, and a `doubt` block per the CLI output contract.
pub fn report(attempt: &Attempt, doubt: &Doubt) -> serde_json::Value {
    serde_json::json!({
        "attempt": {
            "id": attempt.id,
            "parent": attempt.parent,
            "diff": attempt.diff,
            "invertible": attempt.invertible,
            "intent": attempt.intent,
            "recorded_at": attempt.recorded_at.to_rfc3339(),
        },
        "reading": {
            "command": attempt.receipt.command,
            "exit_code": attempt.receipt.exit_code,
            "duration_ms": attempt.receipt.duration.as_millis() as u64,
            "cwd": attempt.receipt.cwd,
        },
        "broken_conditions": attempt
            .broken_conditions()
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "expected": c.expected,
                    "actual": c.actual,
                })
            })
            .collect::<Vec<_>>(),
        "unobserved_conditions": attempt
            .preconditions
            .iter()
            .filter(|c| !c.broken && c.actual.is_none())
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
        "approach_warning": attempt
            .approach_warning
            .as_ref()
            .map(|w| {
                serde_json::json!({
                    "predicted_break": w.predicted_break,
                    "proximity": w.proximity,
                })
            }),
        "doubt": doubt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn receipt(cmd: &str, out: &str, code: i32) -> Receipt {
        Receipt {
            command: cmd.to_string(),
            output: out.to_string(),
            exit_code: Some(code),
            duration: Duration::from_millis(42),
            cwd: Some("/tmp".to_string()),
        }
    }

    fn condition(name: &str, expected: &str, actual: Option<&str>, broken: bool) -> Condition {
        Condition {
            name: name.to_string(),
            expected: expected.to_string(),
            actual: actual.map(|s| s.to_string()),
            broken,
        }
    }

    fn attempt() -> Attempt {
        Attempt {
            id: 7,
            receipt: receipt("cargo test", "FAILED", 101),
            parent: "abc1234".to_string(),
            diff: "diff --git a/src/x.rs b/src/x.rs".to_string(),
            preconditions: vec![
                condition("trunk-root", "abc1234", Some("abc1234"), false),
                condition("lockfile", "pinned", Some("drifted"), true),
                condition("network", "offline", None, false),
            ],
            invertible: true,
            intent: "trying to hit the lockfile drift; got a compile error first".to_string(),
            recorded_at: Utc::now(),
            approach_warning: None,
            status: AttemptStatus::Unjudged,
        }
    }

    fn doubt() -> Doubt {
        Doubt {
            assumptions: vec!["workdir is a git repo".to_string()],
            blind_spots: vec!["network state not observable".to_string()],
            last_validation: "receipt exit code 101".to_string(),
            stale_after: "next trunk commit in touched regions".to_string(),
        }
    }

    #[test]
    fn attempt_constructs_and_broken_conditions_select() {
        let a = attempt();
        let broken = a.broken_conditions();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].name, "lockfile");
    }

    #[test]
    fn judgment_round_trip_through_serde() {
        let j = Judgment {
            annotation: "discard: theory assumed pinned lockfile".to_string(),
            doubt: doubt(),
        };
        let json = serde_json::to_string(&j).unwrap();
        let back: Judgment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, j);
    }

    #[test]
    fn judgment_validate_rejects_missing_annotation_and_doubt() {
        let no_annotation = Judgment {
            annotation: "   ".to_string(),
            doubt: doubt(),
        };
        assert!(no_annotation.validate().is_some());

        let no_doubt = Judgment {
            annotation: "promote: it works".to_string(),
            doubt: Doubt::default(),
        };
        assert!(no_doubt.validate().is_some());

        let good = Judgment {
            annotation: "promote: it works".to_string(),
            doubt: doubt(),
        };
        assert!(good.validate().is_none());
    }

    #[test]
    fn attempt_serde_round_trip_preserves_fields() {
        let mut a = attempt();
        a.approach_warning = Some(ApproachWarning {
            predicted_break: "lockfile drift on next cargo update".to_string(),
            proximity: "same command family".to_string(),
        });
        a.status = AttemptStatus::Judged(Judgment {
            annotation: "discard".to_string(),
            doubt: doubt(),
        });
        let json = serde_json::to_string(&a).unwrap();
        let back: Attempt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, a.id);
        assert_eq!(back.parent, a.parent);
        assert!(back.approach_warning.is_some());
        assert!(matches!(back.status, AttemptStatus::Judged(_)));
    }

    #[test]
    fn report_points_into_the_graph_with_doubt_block() {
        let a = attempt();
        let r = report(&a, &doubt());
        assert_eq!(r["attempt"]["id"], 7);
        assert_eq!(r["attempt"]["parent"], "abc1234");
        assert_eq!(r["reading"]["exit_code"], 101);
        let broken = r["broken_conditions"].as_array().unwrap();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0]["name"], "lockfile");
        // The unobserved condition is tagged, not silently passed.
        let unobserved = r["unobserved_conditions"].as_array().unwrap();
        assert_eq!(unobserved[0], "network");
        assert!(r["doubt"]["blind_spots"].as_array().unwrap().len() == 1);
        assert!(r["approach_warning"].is_null());
    }

    #[test]
    fn doubt_serializes_with_cli_contract_field_names() {
        let d = doubt();
        let json = serde_json::to_value(&d).unwrap();
        assert!(json.get("assumptions").is_some());
        assert!(json.get("blind_spots").is_some());
        assert!(json.get("last_validation").is_some());
        assert!(json.get("stale_after").is_some());
    }
}
