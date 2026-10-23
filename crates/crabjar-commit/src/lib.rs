//! Crabjar Commit Message Generator
//!
//! Generates conventional commit messages from git diffs using heuristics.
//! Can be extended with LLM integration via crabjar-orchestrator.

use crabjar_git::{GitRepo, GitError};
use serde::{Deserialize, Serialize};

/// Commit message type (conventional commits)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitType {
    Feature,
    Fix,
    Docs,
    Style,
    Refactor,
    Test,
    Chore,
    Perf,
    Ci,
    Build,
    Revert,
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitType::Feature => write!(f, "feat"),
            CommitType::Fix => write!(f, "fix"),
            CommitType::Docs => write!(f, "docs"),
            CommitType::Style => write!(f, "style"),
            CommitType::Refactor => write!(f, "refactor"),
            CommitType::Test => write!(f, "test"),
            CommitType::Chore => write!(f, "chore"),
            CommitType::Perf => write!(f, "perf"),
            CommitType::Ci => write!(f, "ci"),
            CommitType::Build => write!(f, "build"),
            CommitType::Revert => write!(f, "revert"),
        }
    }
}

/// Generated commit message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMessage {
    pub type_: CommitType,
    pub scope: Option<String>,
    pub subject: String,
    pub body: Option<String>,
}

impl std::fmt::Display for CommitMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(scope) = &self.scope {
            write!(f, "{}({}): {}", self.type_, scope, self.subject)?;
        } else {
            write!(f, "{}: {}", self.type_, self.subject)?;
        }
        
        if let Some(body) = &self.body {
            write!(f, "\n\n{}", body)?;
        }
        
        Ok(())
    }
}

/// Commit message generator
pub struct CommitGenerator<'a> {
    repo: &'a GitRepo,
}

impl<'a> CommitGenerator<'a> {
    pub fn new(repo: &'a GitRepo) -> Self {
        Self { repo }
    }

    /// Generate a commit message from current staged/unstaged changes
    pub fn generate(&self) -> Result<CommitMessage, GitError> {
        let diffs = self.repo.diff()?;
        
        if diffs.is_empty() {
            return Err(GitError::NoChanges);
        }

        // Analyze the diffs to determine commit type and message
        let (commit_type, scope, subject) = self.analyze_diffs(&diffs);
        
        Ok(CommitMessage {
            type_: commit_type,
            scope,
            subject,
            body: None,
        })
    }

    /// Analyze diffs to determine commit type and generate message
    fn analyze_diffs(&self, diffs: &[crabjar_git::FileDiff]) -> (CommitType, Option<String>, String) {
        // Simple heuristic analysis based on file paths and changes
        
        // Count changes by type
        let mut total_added = 0;
        let mut total_deleted = 0;
        for diff in diffs {
            total_added += diff.added_lines;
            total_deleted += diff.deleted_lines;
        }

        // Determine scope from file paths
        let scope = self.extract_scope(diffs);

        // Generate subject based on changes
        let subject = if diffs.len() == 1 {
            format!("update {}", &diffs[0].path)
        } else {
            format!("update {} files", diffs.len())
        };

        // Determine commit type (heuristic)
        let commit_type = if total_added > total_deleted * 2 {
            CommitType::Feature
        } else if total_deleted > total_added * 2 {
            CommitType::Refactor
        } else {
            CommitType::Chore
        };

        (commit_type, scope, subject)
    }

    /// Extract scope from file paths
    fn extract_scope(&self, diffs: &[crabjar_git::FileDiff]) -> Option<String> {
        // Look for common directory prefix
        if diffs.is_empty() {
            return None;
        }

        let first_path = &diffs[0].path;
        if let Some(slash) = first_path.find('/') {
            let dir = &first_path[..slash];
            
            // Check if all files are in the same directory
            if diffs.iter().all(|d| d.path.starts_with(dir)) {
                return Some(dir.to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_commit_type_display() {
        assert_eq!(CommitType::Feature.to_string(), "feat");
        assert_eq!(CommitType::Fix.to_string(), "fix");
        assert_eq!(CommitType::Docs.to_string(), "docs");
    }

    #[test]
    fn test_commit_message_display() {
        let msg = CommitMessage {
            type_: CommitType::Feature,
            scope: Some("git".to_string()),
            subject: "add commit message generation".to_string(),
            body: None,
        };
        
        assert_eq!(msg.to_string(), "feat(git): add commit message generation");
    }

    #[test]
    fn test_commit_message_with_body() {
        let msg = CommitMessage {
            type_: CommitType::Fix,
            scope: None,
            subject: "resolve lifetime issue".to_string(),
            body: Some("Fixed the parent commit reference lifetime by restructuring\nthe match expression to keep the commit alive for the entire duration.".to_string()),
        };
        
        let output = msg.to_string();
        assert!(output.contains("fix: resolve lifetime issue"));
        assert!(output.contains("Fixed the parent commit reference"));
    }

    #[test]
    fn test_generate_from_real_repo() {
        let repo_path = Path::new("/home/crombo/projects/active/crabjar");
        let repo = GitRepo::open(repo_path).expect("Failed to open repo");
        
        let generator = CommitGenerator::new(&repo);
        // Should not error, even if there are no changes
        let result = generator.generate();
        
        // Result may be Ok or Err(NoChanges) depending on repo state
        match result {
            Ok(msg) => assert!(!msg.subject.is_empty()),
            Err(GitError::NoChanges) => {},
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}
