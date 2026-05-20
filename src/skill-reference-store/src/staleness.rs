#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn retrieve_all_with_no_indexed_returns_empty() {
        let indexed: Vec<serde_json::Value> = vec![];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn retrieve_all_with_empty_path_skips() {
        let dir = tempdir().unwrap();
        let entry = serde_json::json! {
            {
                "path": "",
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn retrieve_all_with_stale_entry_skips() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 0);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn retrieve_all_with_non_stale_returns_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        let retrieved = result.unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn retrieve_all_with_corrupted_entry_errors() {
        let dir = tempdir().unwrap();
        let entry = serde_json::json! {
            {
                "path": dir.path().join("missing.md").to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        assert!(result.is_err());
    }

    #[test]
    fn flag_stale_with_no_indexed_returns_empty() {
        let indexed: Vec<serde_json::Value> = vec![];
        let result = crate::staleness::flag_stale(&indexed, 7);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn flag_stale_with_empty_path_no_flag() {
        let dir = tempdir().unwrap();
        let entry = serde_json::json! {
            {
                "path": "",
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 7);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn flag_stale_with_stale_entry_flags_update() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 0);
        let stale = result.unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0]["action"], "update");
    }

    #[test]
    fn flag_stale_with_non_stale_no_flag() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 7);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn flag_stale_with_corrupted_entry_errors() {
        let dir = tempdir().unwrap();
        let entry = serde_json::json! {
            {
                "path": dir.path().join("missing.md").to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 7);
        assert!(result.is_err());
    }

    #[test]
    fn flag_stale_with_skill_name_preserved() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "edge-test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 0);
        let stale = result.unwrap();
        assert_eq!(stale[0]["skill_name"], "edge-test");
    }

    #[test]
    fn flag_stale_with_type_preserved() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "script"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 0);
        let stale = result.unwrap();
        assert_eq!(stale[0]["type"], "script");
    }

    #[test]
    fn retrieve_all_with_multi_entries_some_stale() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("test1.md");
        std::fs::write(&file1, "content 1\n").unwrap();
        let file2 = dir.path().join("test2.md");
        std::fs::write(&file2, "content 2\n").unwrap();
        let entry1 = serde_json::json! {
            {
                "path": file1.to_string_lossy(),
                "skill_name": "test1",
                "type": "reference"
            }
        };
        let entry2 = serde_json::json! {
            {
                "path": file2.to_string_lossy(),
                "skill_name": "test2",
                "type": "reference"
            }
        };
        let indexed = vec![entry1, entry2];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        let retrieved = result.unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn retrieve_all_with_multi_entries_one_stale() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("test1.md");
        std::fs::write(&file1, "content 1\n").unwrap();
        let file2 = dir.path().join("test2.md");
        std::fs::write(&file2, "content 2\n").unwrap();
        let entry1 = serde_json::json! {
            {
                "path": file1.to_string_lossy(),
                "skill_name": "test1",
                "type": "reference"
            }
        };
        let entry2 = serde_json::json! {
            {
                "path": file2.to_string_lossy(),
                "skill_name": "test2",
                "type": "reference"
            }
        };
        let indexed = vec![entry1, entry2];
        let result = crate::staleness::retrieve_all(&indexed, 0);
        let retrieved = result.unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn retrieve_all_with_path_with_spaces() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test file.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 7);
        let retrieved = result.unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn flag_stale_with_path_with_spaces() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test file.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 0);
        let stale = result.unwrap();
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn retrieve_all_with_staleness_days_large() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 1000);
        let retrieved = result.unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn flag_stale_with_staleness_days_large() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 1000);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn retrieve_all_with_staleness_days_zero() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::retrieve_all(&indexed, 0);
        let retrieved = result.unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn flag_stale_with_staleness_days_zero() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "test content\n").unwrap();
        let entry = serde_json::json! {
            {
                "path": file.to_string_lossy(),
                "skill_name": "test",
                "type": "reference"
            }
        };
        let indexed = vec![entry];
        let result = crate::staleness::flag_stale(&indexed, 0);
        let stale = result.unwrap();
        assert_eq!(stale.len(), 1);
    }
}
