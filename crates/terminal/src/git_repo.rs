//! Git backend for the fine tier (ADR-006, second cut).
//!
//! The attempt graph *is* the git commit graph; this module is the thin
//! shell-out that reads the graph and performs the fine-tier rewind
//! (`git revert`). It is the concrete *outside* the glass: the `Attempt`
//! model and the tier decision live in [`crate::attempts`], and this is
//! where the git mechanics actually run.
//!
//! The reconcilability check (the ADR invariant) is mechanical here:
//!
//! > an attempt is mergeable/revertable iff the trunk hasn't moved in the
//! > regions the attempt touched since the attempt's root.
//!
//! "Regions" = the files the attempt's own commit changed. "Trunk moved"
//! = the commits *after* the attempt (between the attempt and the current
//! HEAD) modified any of those same files. [`GitRepo::trunk_moved_in_regions`]
//! is that check: it is the pre-flight that makes "cleanly revertable" a
//! computed fact, not a hope.
//!
//! No `git2` dependency: the crate already shells out to its backends
//! (herdr, wezterm, zellij), and git's CLI is the stable interface the
//! ADR names (`git revert`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

/// A git working tree the fine tier operates on.
#[derive(Debug, Clone)]
pub struct GitRepo {
    root: PathBuf,
}

impl GitRepo {
    /// Open a git repo at `root`, verifying it is actually a git work tree.
    ///
    /// The fine tier *requires* a git repo (ADR-006: workdirs that aren't
    /// repos fall to the coarse tier). This is the gate that enforces that.
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        let repo = Self {
            root: root.to_path_buf(),
        };
        let inside = repo.run(&["rev-parse", "--is-inside-work-tree"])?;
        if inside.trim() != "true" {
            anyhow::bail!(
                "fine tier requires a git work tree; {} is not one (coarse tier territory)",
                root.display()
            );
        }
        Ok(repo)
    }

    /// Whether `root` is a git work tree (without opening).
    pub fn is_repo(root: &Path) -> bool {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(root)
            .arg("rev-parse")
            .arg("--is-inside-work-tree");
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        cmd.output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    /// The repo root as an absolute path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The current HEAD commit (full sha).
    pub fn head(&self) -> anyhow::Result<String> {
        Ok(self.run(&["rev-parse", "HEAD"])?.trim().to_string())
    }

    /// The first parent of `commit` (`commit^`).
    pub fn parent(&self, commit: &str) -> anyhow::Result<String> {
        Ok(self
            .run(&["rev-parse", &format!("{commit}^")])?
            .trim()
            .to_string())
    }

    /// Whether `a` is an ancestor of `b` (the "the attempt is on this line"
    /// check).
    pub fn is_ancestor(&self, a: &str, b: &str) -> anyhow::Result<bool> {
        let mut cmd = self.cmd(&["merge-base", "--is-ancestor", a, b]);
        let status = cmd.status()?;
        Ok(status.success())
    }

    /// The files changed between commits `a` and `b` (the "regions" a range
    /// touched).
    pub fn diff_names(&self, a: &str, b: &str) -> anyhow::Result<Vec<String>> {
        let out = self.run(&["diff", "--name-only", a, b])?;
        Ok(out
            .lines()
            .map(|l| l.to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    /// The ADR invariant, made mechanical.
    ///
    /// Returns `true` when the trunk has moved in the regions the attempt
    /// touched *since the attempt*: the files the attempt's own commit
    /// changed (its regions) have been modified again by the commits
    /// between the attempt and `trunk`. That means a `git revert` of the
    /// attempt would conflict, so the fine tier must refuse with a
    /// structured refusal — not attempt a best-effort revert.
    ///
    /// `false` (clean) when the trunk's post-attempt work touched disjoint
    /// files, or touched nothing.
    pub fn trunk_moved_in_regions(&self, attempt: &str, trunk: &str) -> anyhow::Result<bool> {
        let parent = self.parent(attempt)?;
        let regions = self.diff_names(&parent, attempt)?;
        if regions.is_empty() {
            return Ok(false);
        }
        let trunk_touched = self.diff_names(attempt, trunk)?;
        Ok(regions.iter().any(|r| trunk_touched.iter().any(|t| t == r)))
    }

    /// The fine-tier rewind: `git revert` of `commit`, applied to the
    /// current HEAD. Returns the new revert commit's sha.
    ///
    /// This is the surgical undo: it records the reversal in history (the
    /// record outlives the understanding) rather than discarding it. The
    /// caller must have run [`Self::trunk_moved_in_regions`] first and
    /// confirmed the trunk did not move in the regions.
    pub fn revert(&self, commit: &str) -> anyhow::Result<String> {
        // `git revert` prints the new short sha in brackets, not the full
        // one; after a successful revert, HEAD *is* the revert commit, so
        // head() is the authoritative answer.
        self.run(&["revert", "--no-edit", commit])?;
        self.head()
    }

    /// The commit subject (first line) — used in the structured report so
    /// the user can see *what* was reverted.
    pub fn commit_subject(&self, commit: &str) -> anyhow::Result<String> {
        Ok(self
            .run(&["log", "-1", "--format=%s", commit])?
            .trim()
            .to_string())
    }

    /// Run a git subcommand in the repo, returning stdout. Bails with the
    /// stderr context on non-zero exit (the herdr.rs pattern).
    fn run(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = self.cmd(args);
        let output = cmd
            .output()
            .with_context(|| format!("failed to execute git {}", args.join(" ")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "git {} failed ({}): {}",
                args.join(" "),
                output.status,
                stderr.trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&self.root).args(args);
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A scratch git repo for real-repo tests: init, identity, no signing.
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
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(&full, content).expect("write");
            run(root, &["add", path]);
            run(root, &["commit", "-q", "-m", &format!("add {path}")]);
            self.repo.head().expect("head")
        }
    }

    /// Run a raw git command in `root`, asserting success.
    fn run(root: &Path, args: &[&str]) {
        let mut c = Command::new("git");
        c.arg("-C").arg(root).args(args);
        let out = c.output().expect("git");
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn open_rejects_non_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(!GitRepo::is_repo(dir.path()));
        assert!(GitRepo::open(dir.path()).is_err());
    }

    #[test]
    fn head_and_ancestor_track_the_graph() {
        let t = TestRepo::new();
        let a = t.commit("a.txt", "a");
        let b = t.commit("b.txt", "b");
        assert_eq!(t.repo.head().unwrap(), b);
        assert!(t.repo.is_ancestor(&a, &b).unwrap());
        assert!(!t.repo.is_ancestor(&b, &a).unwrap());
    }

    #[test]
    fn diff_names_lists_the_regions() {
        let t = TestRepo::new();
        let a = t.commit("a.txt", "a");
        t.commit("b.txt", "b");
        let head = t.repo.head().unwrap();
        let names = t.repo.diff_names(&a, &head).unwrap();
        assert_eq!(names, vec!["b.txt".to_string()]);
    }

    #[test]
    fn trunk_moved_in_regions_is_clean_when_disjoint() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        // The attempt touches only a.txt.
        let attempt = t.commit("a.txt", "a");
        // The trunk then moves, but only in a disjoint file (b.txt).
        let trunk = t.commit("b.txt", "b");
        assert!(!t.repo.trunk_moved_in_regions(&attempt, &trunk).unwrap());
    }

    #[test]
    fn trunk_moved_in_regions_detects_overlap() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        // The attempt touches a.txt.
        let attempt = t.commit("a.txt", "a");
        // The trunk then re-touches a.txt -> overlap -> not cleanly revertable.
        let trunk = t.commit("a.txt", "a-modified-by-trunk");
        assert!(t.repo.trunk_moved_in_regions(&attempt, &trunk).unwrap());
    }

    #[test]
    fn trunk_moved_in_regions_clean_when_trunk_is_the_attempt() {
        let t = TestRepo::new();
        let _base = t.commit("base.txt", "base");
        let attempt = t.commit("a.txt", "a");
        // No trunk movement after the attempt: clean.
        assert!(!t.repo.trunk_moved_in_regions(&attempt, &attempt).unwrap());
    }

    #[test]
    fn revert_undoes_the_delta_and_keeps_history() {
        let t = TestRepo::new();
        let _base = t.commit("keep.txt", "keep");
        let bad = t.commit("bad.txt", "bad-value");
        // Pre-flight: clean (no trunk movement after the attempt).
        assert!(!t.repo.trunk_moved_in_regions(&bad, &bad).unwrap());
        // The revert should remove bad.txt's change.
        let revert_sha = t.repo.revert(&bad).unwrap();
        assert_ne!(revert_sha, bad);
        // bad.txt is gone from the work tree after the revert.
        let root = t.dir.path();
        assert!(!root.join("bad.txt").exists());
        // keep.txt survives (surgical: only the attempt's delta is undone).
        assert!(root.join("keep.txt").exists());
        // History retained the bad commit and its revert (record outlives
        // the understanding).
        let log = {
            let mut c = Command::new("git");
            c.arg("-C").arg(root).arg("log").arg("--oneline");
            let out = c.output().expect("git log");
            String::from_utf8_lossy(&out.stdout).to_string()
        };
        assert!(
            log.lines().count() >= 3,
            "expected base + bad + revert, got:\n{log}"
        );
    }
}
