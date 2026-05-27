use serde_json::json;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct DotfileManager {
    pub project_root: PathBuf,
}

impl DotfileManager {
    /// Create a new DotfileManager instance
    pub fn new(root: PathBuf) -> Self {
        Self { project_root: root }
    }

    /// Generates an rsync promotion plan from staging to target
    pub fn propose(
        &self,
        staging: &str,
        target: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let staging_path = PathBuf::from(staging);
        let target_path = PathBuf::from(target);

        // Validate that the staging directory actually exists before proposing a move
        if !staging_path.exists() {
            return Err(format!("Staging path '{}' does not exist.", staging).into());
        }

        // The promotion command uses rsync with archive mode and deletion of
        // files in target that are no longer present in staging.
        // This ensures the "System Truth" matches the "Agent Staging" exactly.
        let command = format!(
            "rsync -av --delete {}/ {}",
            staging_path.display(),
            target_path.display()
        );

        Ok(json!({
            "success": true,
            "action": "promote_via_rsync",
            "command": command,
            "description": format!("Promote changes from {} to {}", staging, target),
            "safety_check": "Review the rsync command carefully. This will overwrite files in the target directory."
        }))
    }

    /// Performs a lightweight check of the relationship between staging and target
    #[allow(dead_code)]
    pub fn verify(
        &self,
        staging: &str,
        target: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let staging_path = PathBuf::from(staging);
        let target_path = PathBuf::from(target);

        let s_exists = staging_path.exists();
        let t_exists = target_path.exists();

        // In a production version, we would run 'diff -r' or check file hashes here.
        // For this initial implementation, we verify the existence of both nodes in the pipeline.
        Ok(json!({
            "success": true,
            "status": {
                "staging_exists": s_exists,
                "target_exists": t_exists,
                "drift_detected": !s_exists || !t_exists
            },
            "message": if s_exists && t_exists {
                "Both paths are accessible. Ready for promotion."
            } else {
                "One or both paths are missing. Verification failed."
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_creates_manager() {
        let dir = tempdir().unwrap();
        let manager = DotfileManager::new(dir.path().to_path_buf());
        assert_eq!(manager.project_root, dir.path());
    }

    #[test]
    fn test_propose_success() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        let target = dir.path().join("target");
        std::fs::create_dir_all(&staging).unwrap();

        let manager = DotfileManager::new(dir.path().to_path_buf());
        let result = manager.propose(staging.to_str().unwrap(), target.to_str().unwrap());
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["action"], "promote_via_rsync");
        assert!(value["command"].is_string());
        assert!(value["command"].as_str().unwrap().contains("rsync"));
        assert!(value["description"].is_string());
        assert!(value["safety_check"].is_string());
    }

    #[test]
    fn test_propose_staging_not_found() {
        let dir = tempdir().unwrap();
        let manager = DotfileManager::new(dir.path().to_path_buf());
        let result = manager.propose(
            "/nonexistent/staging/path",
            dir.path().join("target").to_str().unwrap(),
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("does not exist"));
    }

    #[test]
    fn test_verify_both_exist() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        let target = dir.path().join("target");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let manager = DotfileManager::new(dir.path().to_path_buf());
        let result = manager.verify(staging.to_str().unwrap(), target.to_str().unwrap());
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["status"]["staging_exists"], true);
        assert_eq!(value["status"]["target_exists"], true);
        assert_eq!(value["status"]["drift_detected"], false);
        assert_eq!(
            value["message"],
            "Both paths are accessible. Ready for promotion."
        );
    }

    #[test]
    fn test_verify_staging_missing() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();

        let manager = DotfileManager::new(dir.path().to_path_buf());
        let result = manager.verify("/nonexistent/staging", target.to_str().unwrap());
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["status"]["staging_exists"], false);
        assert_eq!(value["status"]["target_exists"], true);
        assert_eq!(value["status"]["drift_detected"], true);
    }

    #[test]
    fn test_verify_target_missing() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();

        let manager = DotfileManager::new(dir.path().to_path_buf());
        let result = manager.verify(staging.to_str().unwrap(), "/nonexistent/target");
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["status"]["staging_exists"], true);
        assert_eq!(value["status"]["target_exists"], false);
        assert_eq!(value["status"]["drift_detected"], true);
    }

    #[test]
    fn test_verify_both_missing() {
        let manager = DotfileManager::new("/tmp".into());
        let result = manager.verify("/nonexistent/staging1", "/nonexistent/staging2");
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["status"]["staging_exists"], false);
        assert_eq!(value["status"]["target_exists"], false);
        assert_eq!(value["status"]["drift_detected"], true);
        assert_eq!(
            value["message"],
            "One or both paths are missing. Verification failed."
        );
    }
}
