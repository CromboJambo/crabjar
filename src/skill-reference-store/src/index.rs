#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn index_all_with_no_skills_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_no_refs_dir_returns_scripts_only() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("test.sh"), "#!/usr/bin/env bash\necho test\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "script");
    }

    #[test]
    fn index_all_with_refs_and_scripts_returns_both() {
        let dir = tempdir().unwrap();
        let refs_dir = dir.path().join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("test.md"), "ref content\n").unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("test.sh"), "#!/usr/bin/env bash\necho test\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn index_all_with_refs_only_returns_refs() {
        let dir = tempdir().unwrap();
        let refs_dir = dir.path().join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("test.md"), "ref content\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0]["skill_name"].is_string());
    }

    #[test]
    fn index_all_with_scripts_only_returns_scripts() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("test.sh"), "#!/usr/bin/env bash\necho test\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "script");
    }

    #[test]
    fn index_all_with_multiple_scripts_returns_all() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("test1.sh"), "#!/usr/bin/env bash\necho test1\n").unwrap();
        std::fs::write(scripts_dir.join("test2.sh"), "#!/usr/bin/env bash\necho test2\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn index_all_with_multiple_refs_returns_all() {
        let dir = tempdir().unwrap();
        let refs_dir = dir.path().join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("test1.md"), "ref 1\n").unwrap();
        std::fs::write(refs_dir.join("test2.md"), "ref 2\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn index_all_with_empty_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_nested_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let result = crate::index_all(&nested).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_refs_dir_not_dir_returns_empty_refs() {
        let dir = tempdir().unwrap();
        let refs_path = dir.path().join("references");
        std::fs::write(&refs_path, "not a dir\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_scripts_dir_not_dir_returns_empty_scripts() {
        let dir = tempdir().unwrap();
        let scripts_path = dir.path().join("scripts");
        std::fs::write(&scripts_path, "not a dir\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_file_in_refs_not_file_skips() {
        let dir = tempdir().unwrap();
        let refs_dir = dir.path().join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        let nested = refs_dir.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_file_in_scripts_not_file_skips() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        let nested = scripts_dir.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_skill_dir_with_name() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("edge-test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let refs_dir = skill_dir.join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("test.md"), "ref content\n").unwrap();
        let result = crate::index_all(&skill_dir).unwrap();
        assert_eq!(result[0]["skill_name"], "edge-test-skill");
    }

    #[test]
    fn index_all_with_skill_dir_no_name() {
        let dir = tempdir().unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn index_all_with_large_script_line_count() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        let lines: Vec<String> = (0..1000).map(|i| format!("line {}", i)).collect();
        std::fs::write(scripts_dir.join("large.sh"), lines.join("\n")).unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result[0]["line_count"], 1000);
    }

    #[test]
    fn index_all_with_empty_script_line_count() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("empty.sh"), "").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result[0]["line_count"], 0);
    }

    #[test]
    fn index_all_with_script_with_spaces_in_name() {
        let dir = tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("test script.sh"), "#!/usr/bin/env bash\necho test\n").unwrap();
        let result = crate::index_all(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
    }
}
