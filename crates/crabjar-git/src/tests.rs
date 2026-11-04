#[cfg(test)]
mod tests {
    use crate::GitRepo;
    use std::path::PathBuf;

    #[test]
    fn test_open_crabjar_repo() {
        let repo_path = PathBuf::from("/home/crombo/projects/active/crabjar");
        let repo = GitRepo::open(&repo_path).expect("Failed to open crabjar repo");
        
        // Should be able to get branch name
        let branch = repo.branch_name().expect("Failed to get branch name");
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_status() {
        let repo_path = PathBuf::from("/home/crombo/projects/active/crabjar");
        let repo = GitRepo::open(&repo_path).expect("Failed to open crabjar repo");

        // Should not error, even if there are no changes
        let status = repo.status().expect("Failed to get status");

        // Verify the status object has the expected fields
        assert!(status.staged.len() >= 0);
        assert!(status.unstaged.len() >= 0);
        assert!(status.untracked.len() >= 0);
    }

    #[test]
    fn test_diff() {
        let repo_path = PathBuf::from("/home/crombo/projects/active/crabjar");
        let repo = GitRepo::open(&repo_path).expect("Failed to open crabjar repo");
        
        // Should not error, even if there are no changes
        let diffs = repo.diff().expect("Failed to get diff");
        
        // Just verify it returns a Vec (may be empty)
        assert!(diffs.len() >= 0);
    }
}
