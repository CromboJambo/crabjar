//! Crabjar Git Operations
//!
//! Provides git operations needed by the crabjar IDE: status, diff, add, commit.
//! Uses libgit2 (via git2 crate) for structural operations and shell out for
//! diff generation to avoid API complexity.

use chrono::Utc;
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
mod tests;

/// Error types for crabjar-git operations
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Repository not found at '{0}'")]
    RepoNotFound(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Shell command failed: {0}\nOutput: {1}")]
    ShellFailed(String, String),

    #[error("Path not in repository: {0}")]
    PathNotInRepo(String),

    #[error("No changes to commit")]
    NoChanges,

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Status of a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub staged: bool,
    pub modified: bool,
    pub deleted: bool,
    pub new_file: bool,
    pub renamed: bool,
}

/// Repository status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<FileStatus>,
}

/// Diff for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub added_lines: usize,
    pub deleted_lines: usize,
    pub hunks: Vec<DiffHunk>,
}

/// A single hunk in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

/// Commit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub oid: String,
    pub message: String,
    pub timestamp: i64,
    pub files_changed: usize,
}

/// Crabjar Git Repository wrapper
pub struct GitRepo {
    repo: Repository,
    root_path: std::path::PathBuf,
}

impl GitRepo {
    /// Open a git repository at the given path
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::open(path).map_err(|_| GitError::RepoNotFound(path.display().to_string()))?;
        let root_path = repo.workdir().unwrap_or(path).to_path_buf();
        Ok(Self { repo, root_path })
    }

    /// Get repository status (staged, unstaged, untracked files)
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        options.show(git2::StatusShow::IndexAndWorkdir);

        let statuses = self.repo.statuses(Some(&mut options))?;

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("(unknown)").to_string();
            let status = entry.status();

            // git2 0.19 uses bitfield constants directly
            let is_staged = status.contains(git2::Status::INDEX_MODIFIED)
                || status.contains(git2::Status::INDEX_NEW)
                || status.contains(git2::Status::INDEX_DELETED);
            let is_modified = status.contains(git2::Status::WT_MODIFIED);
            let is_deleted = status.contains(git2::Status::WT_DELETED);
            let is_new = status.contains(git2::Status::WT_NEW);

            let file_status = FileStatus {
                path: path.clone(),
                staged: is_staged,
                modified: is_modified,
                deleted: is_deleted,
                new_file: is_new,
                renamed: false, // Simplified for now
            };

            if is_staged {
                staged.push(file_status);
            } else if is_modified || is_deleted || is_new {
                unstaged.push(file_status);
            }

            if is_new && !is_staged {
                untracked.push(FileStatus {
                    path,
                    staged: false,
                    modified: false,
                    deleted: false,
                    new_file: true,
                    renamed: false,
                });
            }
        }

        Ok(RepoStatus {
            staged,
            unstaged,
            untracked,
        })
    }

    /// Get diff for all changes (staged and unstaged) using git CLI
    pub fn diff(&self) -> Result<Vec<FileDiff>, GitError> {
        let output = std::process::Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(&self.root_path)
            .output()
            .map_err(|e| GitError::ShellFailed("git diff HEAD".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::ShellFailed("git diff HEAD".to_string(), stderr));
        }

        let diff_text = String::from_utf8_lossy(&output.stdout);
        Ok(self.parse_diff_output(&diff_text))
    }

    /// Stage specific files
    pub fn add(&self, paths: &[String]) -> Result<(), GitError> {
        let mut index = self.repo.index()?;

        for path in paths {
            index.add_path(Path::new(path))?;
        }

        index.write()?;
        Ok(())
    }

    /// Stage all changes
    pub fn add_all(&self) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Create a commit with the given message
    pub fn commit(&self, message: &str, author_name: &str, author_email: &str) -> Result<CommitResult, GitError> {
        let mut index = self.repo.index()?;

        // Check if there are changes to commit
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let now = Utc::now();
        let time = git2::Time::new(now.timestamp(), 0);
        let signature = git2::Signature::new(author_name, author_email, &time)?;

        // Create commit with or without parent
        let commit_oid = match self.repo.head() {
            Ok(head) => {
                if let Ok(parent_commit) = head.peel_to_commit() {
                    self.repo.commit(
                        Some("HEAD"),
                        &signature,
                        &signature,
                        message,
                        &tree,
                        &[&parent_commit],
                    )?
                } else {
                    // No parent (initial commit)
                    self.repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?
                }
            }
            Err(_) => {
                // No HEAD yet (initial commit)
                self.repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?
            }
        };

        // Count files changed
        let status = self.status()?;
        let files_changed = status.staged.len() + status.unstaged.len();

        Ok(CommitResult {
            oid: commit_oid.to_string(),
            message: message.to_string(),
            timestamp: now.timestamp(),
            files_changed,
        })
    }

    /// Get the current branch name
    pub fn branch_name(&self) -> Result<String, GitError> {
        let head = self.repo.head()?;
        Ok(head.shorthand().unwrap_or("(detached)").to_string())
    }

    /// Parse git diff output into structured FileDiff objects
    fn parse_diff_output(&self, diff_text: &str) -> Vec<FileDiff> {
        let mut diffs = Vec::new();
        let lines: Vec<&str> = diff_text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("diff --git") {
                // Parse file header
                let path = self.extract_path_from_diff_header(lines[i]);
                let old_path = None; // Simplified for now

                let mut added = 0;
                let mut deleted = 0;
                let mut hunks = Vec::new();
                let mut current_hunk: Option<DiffHunk> = None;

                i += 1;
                while i < lines.len() && !lines[i].starts_with("diff --git") {
                    if lines[i].starts_with("@@ ") {
                        // New hunk
                        if let Some(h) = current_hunk.take() {
                            hunks.push(h);
                        }
                        current_hunk = Some(DiffHunk {
                            header: lines[i].to_string(),
                            lines: Vec::new(),
                        });
                    } else if let Some(ref mut h) = current_hunk {
                        // Parse diff line
                        let kind = match lines[i].chars().next() {
                            Some('+') => DiffLineKind::Addition,
                            Some('-') => DiffLineKind::Deletion,
                            _ => DiffLineKind::Context,
                        };

                        if matches!(kind, DiffLineKind::Addition) {
                            added += 1;
                        } else if matches!(kind, DiffLineKind::Deletion) {
                            deleted += 1;
                        }

                        h.lines.push(DiffLine {
                            kind,
                            content: lines[i].to_string(),
                        });
                    }
                    i += 1;
                }

                if let Some(h) = current_hunk.take() {
                    hunks.push(h);
                }

                diffs.push(FileDiff {
                    path,
                    old_path,
                    added_lines: added,
                    deleted_lines: deleted,
                    hunks,
                });
            } else {
                i += 1;
            }
        }

        diffs
    }

    /// Extract file path from diff header line
    fn extract_path_from_diff_header(&self, header: &str) -> String {
        // Format: "diff --git a/path b/path" or "diff --git a/old b/new"
        if let Some(b_part) = header.find("b/") {
            return header[b_part + 2..].trim().to_string();
        }
        if let Some(a_part) = header.find("a/") {
            return header[a_part + 2..].trim().to_string();
        }
        "(unknown)".to_string()
    }
}
