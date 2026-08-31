//! Rewind orchestration (ADR-006, second cut) — the tier router.
//!
//! The [`Attempt`] records the tier decision ([`RewindTier`]); this module
//! *executes* the fine tier and refuses everything else with a structured
//! reason. Rewind is **user-initiated, never agent-auto** (ADR-006): the
//! user chooses which point to bring state back to.
//!
//! The fine-tier pre-flight makes "cleanly revertable" a computed fact, not
//! a hope — the ADR invariant, mechanical:
//!
//! > an attempt is mergeable/revertable iff the trunk hasn't moved in the
//! > regions the attempt touched since the attempt's root.
//!
//! Two pre-flight checks gate the revert:
//!
//! 1. **Line check** — the attempt is an ancestor of the trunk. An attempt
//!    that is not on this line has no defined revert (its delta may already
//!    be woven into later commits); refuse, do not guess.
//! 2. **Region check** — [`GitRepo::trunk_moved_in_regions`]: the files the
//!    attempt's own commit changed have not been re-modified by the commits
//!    between the attempt and the trunk. Overlap means the revert would
//!    conflict; refuse with the overlapping regions in hand.
//!
//! The coarse tier (VM destroy + restore) is *recorded, not executed*: its
//! home is the habitat VM layer (ADR-002/003), not yet live. A coarse-tier
//! rewind is refused with [`RewindRefusalReason::CoarseTierNotLive`] — the
//! decision is in the record, the execution is deferred to that layer.

use crate::attempts::{Attempt, RewindTier};
use crate::git_repo::GitRepo;
use std::path::Path;

/// Why a rewind was refused. A refusal is a value, not a panic — the
/// caller (CLI, user) gets the reason in hand and the state is untouched.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewindRefusalReason {
    /// The attempt is non-invertible (`rm -rf`, `git push`, migrations):
    /// no clean local inverse. The coarse tier is the only path, and it
    /// is not live yet.
    CoarseTierNotLive,
    /// The fine tier requires a git work tree; the workdir is not one.
    /// Coarse tier territory (per the ADR's workdir assumption).
    NotAGitRepo,
    /// The attempt commit is not an ancestor of the trunk: no defined
    /// revert on this line.
    AttemptNotOnLine,
    /// The trunk moved in the regions the attempt touched since the
    /// attempt: a `git revert` would conflict. The overlapping regions
    /// are reported so the user can see *why*.
    TrunkMovedInRegions,
}

/// A structured refusal: the rewind did not happen, and here is exactly
/// why. The work tree is untouched.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RewindRefusal {
    /// The machine-readable reason.
    pub reason: RewindRefusalReason,
    /// The tier the attempt routes to (what the refusal is about).
    pub tier: RewindTier,
    /// Human-readable detail (the ADR's "pointers into the graph"):
    /// overlapping regions, the offending commit, ...
    pub detail: String,
}

/// The outcome of a fine-tier rewind.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RewindOutcome {
    /// The revert landed: the new revert commit's sha, the subject of the
    /// reverted commit, and the regions that were undone. The reversal is
    /// in history (the record outlives the understanding).
    Reverted {
        /// The new revert commit (HEAD after the revert).
        revert_commit: String,
        /// The subject of the reverted attempt commit.
        subject: String,
        /// The regions the revert undid (the attempt's changed files).
        regions: Vec<String>,
    },
    /// The rewind was refused; the work tree is untouched.
    Refused(RewindRefusal),
}

impl RewindOutcome {
    /// Whether the rewind actually rewound.
    pub fn succeeded(&self) -> bool {
        matches!(self, RewindOutcome::Reverted { .. })
    }
}

/// Pre-flight verdict for a fine-tier rewind — the computed fact behind
/// "cleanly revertable" (or "cleanly not").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewindPreflight {
    /// The revert is clean: the attempt is on the line and the trunk did
    /// not move in its regions.
    Clean,
    /// The attempt is not an ancestor of the trunk.
    NotOnLine,
    /// The trunk moved in the attempt's regions; the overlapping files.
    TrunkMoved(Vec<String>),
}

/// The fine-tier pre-flight, as a pure function of the graph.
///
/// This is the check the ADR makes mechanical; separating it from the
/// revert keeps the refusal path testable and lets the CLI expose a
/// dry-run (verdict without mutation).
pub fn preflight(repo: &GitRepo, attempt: &str, trunk: &str) -> anyhow::Result<RewindPreflight> {
    if !repo.is_ancestor(attempt, trunk)? {
        return Ok(RewindPreflight::NotOnLine);
    }
    if repo.trunk_moved_in_regions(attempt, trunk)? {
        let parent = repo.parent(attempt)?;
        let regions = repo.diff_names(&parent, attempt)?;
        let trunk_touched = repo.diff_names(attempt, trunk)?;
        let overlap: Vec<String> = regions
            .iter()
            .filter(|r| trunk_touched.iter().any(|t| t == *r))
            .cloned()
            .collect();
        return Ok(RewindPreflight::TrunkMoved(overlap));
    }
    Ok(RewindPreflight::Clean)
}

/// Execute (or refuse) the rewind of `attempt` in the repo at `workdir`.
///
/// `trunk` is the line the attempt must sit on — normally the current
/// HEAD of the work tree. Routing:
///
/// - **Coarse tier** (non-invertible): refused — the VM layer is not
///   live; the decision stays in the record.
/// - **Fine tier**: requires a git work tree, passes both pre-flight
///   checks, then `git revert`s the attempt's commit.
///
/// Refusals never touch the work tree; a failed `git revert` itself
/// surfaces as an `Err` (the revert is atomic per commit — git restores
/// the index on conflict, so the caller can retry after triage).
pub fn rewind(attempt: &Attempt, workdir: &Path, trunk: &str) -> anyhow::Result<RewindOutcome> {
    if attempt.tier() == RewindTier::Coarse {
        return Ok(RewindOutcome::Refused(RewindRefusal {
            reason: RewindRefusalReason::CoarseTierNotLive,
            tier: RewindTier::Coarse,
            detail: format!(
                "attempt {} is non-invertible; the coarse tier (VM destroy + restore) \
                 is the habitat VM layer (ADR-002/003) and is not yet live — the \
                 decision is recorded, the execution is deferred",
                attempt.id
            ),
        }));
    }

    let repo = match GitRepo::open(workdir) {
        Ok(repo) => repo,
        Err(_) => {
            return Ok(RewindOutcome::Refused(RewindRefusal {
                reason: RewindRefusalReason::NotAGitRepo,
                tier: RewindTier::Fine,
                detail: format!(
                    "{} is not a git work tree; the fine tier requires one (coarse tier \
                     territory)",
                    workdir.display()
                ),
            }));
        }
    };

    match preflight(&repo, &attempt.parent, trunk)? {
        RewindPreflight::NotOnLine => Ok(RewindOutcome::Refused(RewindRefusal {
            reason: RewindRefusalReason::AttemptNotOnLine,
            tier: RewindTier::Fine,
            detail: format!(
                "attempt commit {} is not an ancestor of trunk {trunk}; no defined \
                 revert on this line",
                attempt.parent
            ),
        })),
        RewindPreflight::TrunkMoved(overlap) => Ok(RewindOutcome::Refused(RewindRefusal {
            reason: RewindRefusalReason::TrunkMovedInRegions,
            tier: RewindTier::Fine,
            detail: format!(
                "trunk moved in {} region(s) the attempt touched: {}",
                overlap.len(),
                overlap.join(", ")
            ),
        })),
        RewindPreflight::Clean => {
            let parent = repo.parent(&attempt.parent)?;
            let regions = repo.diff_names(&parent, &attempt.parent)?;
            let subject = repo.commit_subject(&attempt.parent)?;
            let revert_commit = repo.revert(&attempt.parent)?;
            Ok(RewindOutcome::Reverted {
                revert_commit,
                subject,
                regions,
            })
        }
    }
}

/// The pre-flight verdict for a commit, without any mutation — the
/// dry-run path. `None` when the workdir is not a git work tree (the
/// caller reports that as a coarse-tier refusal).
pub fn preflight_commit(
    workdir: &Path,
    commit: &str,
    trunk: &str,
) -> anyhow::Result<Option<RewindPreflight>> {
    let repo = match GitRepo::open(workdir) {
        Ok(repo) => repo,
        Err(_) => return Ok(None),
    };
    preflight(&repo, commit, trunk).map(Some)
}

/// Rewind a commit directly (the CLI entry point).
///
/// The CLI operates on commits, not queue entries — the user points at
/// the commit to undo. The commit is treated as an invertible attempt
/// (fine tier): the work tree must be a git repo, and both pre-flight
/// checks must pass. `trunk` is the line the commit must sit on —
/// normally the current HEAD.
pub fn rewind_commit(commit: &str, workdir: &Path, trunk: &str) -> anyhow::Result<RewindOutcome> {
    let attempt = Attempt {
        id: 0,
        receipt: crate::stream::Receipt {
            command: String::new(),
            output: String::new(),
            exit_code: None,
            duration: std::time::Duration::ZERO,
            cwd: None,
        },
        parent: commit.to_string(),
        diff: String::new(),
        preconditions: Vec::new(),
        invertible: true,
        intent: String::new(),
        recorded_at: chrono::Utc::now(),
        approach_warning: None,
        status: crate::attempts::AttemptStatus::Unjudged,
    };
    rewind(&attempt, workdir, trunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attempts::{AttemptStatus, Condition};
    use crate::stream::Receipt;
    use chrono::Utc;
    use std::time::Duration;

    /// A scratch git repo for real-repo tests.
    struct TestRepo {
        dir: tempfile::TempDir,
        repo: GitRepo,
    }

    impl TestRepo {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path();
            run(root, &["init", "-q"]);
            run(root, &["config", "user.email", "test@crabjar.local"]);
            run(root, &["config", "user.name", "crabjar-test"]);
            run(root, &["config", "commit.gpgsign", "false"]);
            let repo = GitRepo::open(root).expect("open repo");
            Self { dir, repo }
        }

        /// Write `path` with `content` and commit it; return the sha.
        fn commit(&self, path: &str, content: &str) -> String {
            let root = self.dir.path();
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("mkdir");
            }
            std::fs::write(&full, content).expect("write");
            run(root, &["add", path]);
            run(root, &["commit", "-q", "-m", &format!("add {path}")]);
            self.repo.head().expect("head")
        }
    }

    fn run(root: &Path, args: &[&str]) {
        let mut c = std::process::Command::new("git");
        c.arg("-C").arg(root).args(args);
        let out = c.output().expect("git");
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// An attempt rooted at `parent`, on the fine tier by default.
    fn attempt(id: u64, parent: &str, invertible: bool) -> Attempt {
        Attempt {
            id,
            receipt: Receipt {
                command: "cargo test".to_string(),
                output: String::new(),
                exit_code: Some(101),
                duration: Duration::from_millis(1),
                cwd: None,
            },
            parent: parent.to_string(),
            diff: "diff".to_string(),
            preconditions: vec![Condition {
                name: "trunk-root".to_string(),
                expected: parent.to_string(),
                actual: Some(parent.to_string()),
                broken: false,
            }],
            invertible,
            intent: "trying to hit the wall".to_string(),
            recorded_at: Utc::now(),
            approach_warning: None,
            status: AttemptStatus::Unjudged,
        }
    }

    #[test]
    fn preflight_clean_on_disjoint_trunk_movement() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let attempt = t.commit("a.txt", "a");
        let trunk = t.commit("b.txt", "b");
        assert_eq!(
            preflight(&t.repo, &attempt, &trunk).unwrap(),
            RewindPreflight::Clean
        );
    }

    #[test]
    fn preflight_detects_region_overlap() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let attempt = t.commit("a.txt", "a");
        let trunk = t.commit("a.txt", "a-modified");
        match preflight(&t.repo, &attempt, &trunk).unwrap() {
            RewindPreflight::TrunkMoved(overlap) => {
                assert_eq!(overlap, vec!["a.txt".to_string()]);
            }
            other => panic!("expected TrunkMoved, got {other:?}"),
        }
    }

    #[test]
    fn preflight_refuses_attempt_not_on_line() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let a = t.commit("a.txt", "a");
        let b = t.commit("b.txt", "b");
        // b is not an ancestor of a: wrong direction on the line.
        assert_eq!(
            preflight(&t.repo, &b, &a).unwrap(),
            RewindPreflight::NotOnLine
        );
    }

    #[test]
    fn rewind_reverts_a_clean_fine_tier_attempt() {
        let t = TestRepo::new();
        let _base = t.commit("keep.txt", "keep");
        let bad = t.commit("bad.txt", "bad-value");
        let att = attempt(1, &bad, true);
        let outcome = rewind(&att, t.dir.path(), &bad).unwrap();
        match &outcome {
            RewindOutcome::Reverted {
                revert_commit,
                subject,
                regions,
            } => {
                assert_ne!(*revert_commit, bad);
                assert!(subject.contains("bad.txt"));
                assert_eq!(*regions, vec!["bad.txt".to_string()]);
            }
            other => panic!("expected Reverted, got {other:?}"),
        }
        assert!(!t.dir.path().join("bad.txt").exists());
        assert!(t.dir.path().join("keep.txt").exists());
        assert!(outcome.succeeded());
    }

    #[test]
    fn rewind_refuses_coarse_tier_without_touching_the_tree() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let bad = t.commit("bad.txt", "bad-value");
        let a = attempt(1, &bad, false);
        match rewind(&a, t.dir.path(), &bad).unwrap() {
            RewindOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, RewindRefusalReason::CoarseTierNotLive);
                assert_eq!(refusal.tier, RewindTier::Coarse);
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        // The tree is untouched: bad.txt is still there, no revert commit.
        assert!(t.dir.path().join("bad.txt").exists());
        assert_eq!(t.repo.head().unwrap(), bad);
    }

    #[test]
    fn rewind_refuses_trunk_movement_in_regions() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let att = t.commit("a.txt", "a");
        let trunk = t.commit("a.txt", "a-modified");
        let a = attempt(1, &att, true);
        match rewind(&a, t.dir.path(), &trunk).unwrap() {
            RewindOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, RewindRefusalReason::TrunkMovedInRegions);
                assert!(refusal.detail.contains("a.txt"));
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        // No revert happened: HEAD is still the trunk commit.
        assert_eq!(t.repo.head().unwrap(), trunk);
    }

    #[test]
    fn rewind_refuses_non_repo_workdir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = attempt(1, "abc1234", true);
        match rewind(&a, dir.path(), "abc1234").unwrap() {
            RewindOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, RewindRefusalReason::NotAGitRepo);
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    #[test]
    fn refusal_serializes_with_reason_and_tier() {
        let refusal = RewindRefusal {
            reason: RewindRefusalReason::TrunkMovedInRegions,
            tier: RewindTier::Fine,
            detail: "a.txt".to_string(),
        };
        let json = serde_json::to_value(&refusal).unwrap();
        assert_eq!(json["reason"], "trunk_moved_in_regions");
        assert_eq!(json["tier"], "Fine");
    }
}
