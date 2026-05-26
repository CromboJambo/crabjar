use anyhow::Result;
use serde_json::Value;

use crate::execute_script;

/// Execute a skill script with default environment.
pub fn execute_default(
    script_path: &std::path::Path,
    args: &[String],
    work_dir: &std::path::Path,
) -> Result<Value> {
    let env = std::collections::HashMap::from_iter([
        (
            "HOME".to_string(),
            std::env::var("HOME").unwrap_or_default(),
        ),
        ("PWD".to_string(), std::env::var("PWD").unwrap_or_default()),
    ]);

    let allowlist = std::collections::HashSet::from_iter([script_path.to_owned()]);

    let timeout = std::time::Duration::from_secs(30);

    tokio::runtime::Runtime::new()?.block_on(execute_script(
        script_path,
        args,
        env,
        work_dir,
        timeout,
        &allowlist,
    ))
}

/// Execute multiple scripts in parallel.
pub async fn execute_parallel(scripts: &[(std::path::PathBuf, Vec<String>)]) -> Result<Vec<Value>> {
    let mut handles = Vec::new();

    #[allow(clippy::unnecessary_to_owned)]
    for (path, args) in scripts.iter().cloned() {
        let env = std::collections::HashMap::from_iter([
            (
                "HOME".to_string(),
                std::env::var("HOME").unwrap_or_default(),
            ),
            ("PWD".to_string(), std::env::var("PWD").unwrap_or_default()),
        ]);
        handles.push(tokio::spawn(async move {
            let allowlist = std::collections::HashSet::from_iter([path.clone()]);
            let timeout = std::time::Duration::from_secs(30);
            execute_script(&path, &args, env, &path, timeout, &allowlist).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await?;
        results.push(result?);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn execute_default_runs_echo() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("echo.sh");
        std::fs::write(
            &script_path,
            "#!/bin/bash\necho '{\"result\":\"ok\"}'",
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let work_dir = dir.path();
        let result = execute_default(&script_path, &[], work_dir).unwrap();
        assert_eq!(result["result"], "ok");
    }

    #[test]
    fn execute_default_with_args() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("args.sh");
        std::fs::write(
            &script_path,
            "#!/bin/bash\necho '{\"arg1\":\"$1\",\"arg2\":\"$2\"}'",
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let work_dir = dir.path();
        let result = execute_default(&script_path, &["hello".to_string(), "world".to_string()], work_dir).unwrap();
        assert_eq!(result["arg1"], "hello");
        assert_eq!(result["arg2"], "world");
    }

    #[test]
    fn execute_default_fails_on_nonexistent_script() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("nonexistent.sh");

        let work_dir = dir.path();
        let result = execute_default(&script_path, &[], work_dir);
        assert!(result.is_err());
    }

    #[test]
    fn execute_default_fails_on_non_json_output() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("not-json.sh");
        std::fs::write(&script_path, "#!/bin/bash\necho not-json").unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let work_dir = dir.path();
        let result = execute_default(&script_path, &[], work_dir);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_parallel_runs_multiple() {
        let dir = tempdir().unwrap();

        let script1 = dir.path().join("s1.sh");
        std::fs::write(&script1, "#!/bin/bash\necho '{\"id\":1}'").unwrap();
        std::fs::set_permissions(&script1, std::fs::Permissions::from_mode(0o755)).unwrap();

        let script2 = dir.path().join("s2.sh");
        std::fs::write(&script2, "#!/bin/bash\necho '{\"id\":2}'").unwrap();
        std::fs::set_permissions(&script2, std::fs::Permissions::from_mode(0o755)).unwrap();

        let scripts = vec![
            (script1.clone(), Vec::new()),
            (script2.clone(), Vec::new()),
        ];
        let results = execute_parallel(&scripts).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["id"], 1);
        assert_eq!(results[1]["id"], 2);
    }

    #[tokio::test]
    async fn execute_parallel_empty_input() {
        let scripts: Vec<(std::path::PathBuf, Vec<String>)> = vec![];
        let results = execute_parallel(&scripts).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn execute_parallel_one_fails() {
        let dir = tempdir().unwrap();

        let good_script = dir.path().join("good.sh");
        std::fs::write(&good_script, "#!/bin/bash\necho '{\"ok\":true}'").unwrap();
        std::fs::set_permissions(&good_script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let bad_script = dir.path().join("bad.sh");
        std::fs::write(&bad_script, "#!/bin/bash\nexit 1").unwrap();
        std::fs::set_permissions(&bad_script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let scripts = vec![
            (good_script, Vec::new()),
            (bad_script, Vec::new()),
        ];
        let result = execute_parallel(&scripts).await;
        assert!(result.is_err());
    }
}
