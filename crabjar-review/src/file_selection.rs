use crate::types::{ChangedFile, FileStatus, FileType};
use regex::Regex;
use std::path::Path;

/// Default patterns for generated files to exclude.
const GENERATED_PATTERNS: &[&str] = &[
    r"target/",
    r"build/",
    r"\.lock$",
    r"Cargo\.lock$",
    r"pnpm-lock\.yaml$",
    r"package-lock\.json$",
    r"yarn\.lock$",
];

/// Default patterns for binary assets to exclude.
const BINARY_PATTERNS: &[&str] = &[
    r"\.(png|jpg|jpeg|gif|ico|svg)$",
    r"\.(pdf|docx|xlsx|pptx)$",
    r"\.(zip|tar|gz|bz2|xz)$",
];

/// Classify a file by type based on path and content heuristics.
pub fn classify_file(path: &Path, content: Option<&str>) -> FileType {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let path_str = path.to_string_lossy();

    // Test files
    if path_str.contains("/tests/")
        || path_str.contains("_test.rs")
        || path_str.contains(".test.")
        || path_str.contains(".spec.")
    {
        return FileType::Test;
    }

    // Config files
    match ext {
        "toml" | "yaml" | "yml" | "json" | "ini" | "cfg" | "conf" => {
            if path_str.contains("Cargo.toml") && !path_str.contains("/tests/") {
                return FileType::Config;
            }
            return FileType::Config;
        }
        _ => {}
    }

    // Documentation
    match ext {
        "md" | "rst" | "txt" => return FileType::Docs,
        _ => {}
    }

    // Source files by extension
    match ext {
        "rs" | "c" | "cpp" | "h" | "hpp" | "go" | "py" | "js" | "ts" | "java" | "swift" | "kt" => {
            return FileType::Source;
        }
        _ => {}
    }

    // Content heuristics for ambiguous files
    if let Some(content) = content {
        if content.starts_with("#!/") || content.contains("fn ") || content.contains("def ") {
            return FileType::Source;
        }
        if content.trim().is_empty() {
            return FileType::Unknown;
        }
    }

    FileType::Unknown
}

/// Check if a file matches generated file patterns.
pub fn is_generated(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    for pattern in GENERATED_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&path_str) {
                return true;
            }
        }
    }
    false
}

/// Check if a file matches binary asset patterns.
pub fn is_binary_asset(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    for pattern in BINARY_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&path_str) {
                return true;
            }
        }
    }
    false
}

/// Get changed files from a git diff between two commits.
pub fn get_changed_files(
    repo_path: &Path,
    base_commit: &str,
    target_commit: &str,
) -> Result<Vec<ChangedFile>, String> {
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| format!("Failed to open repository at {:?}: {}", repo_path, e))?;

    let base_obj = repo
        .revparse_single(base_commit)
        .map_err(|e| format!("Failed to resolve base commit '{}': {}", base_commit, e))?;
    let target_obj = repo
        .revparse_single(target_commit)
        .map_err(|e| format!("Failed to resolve target commit '{}': {}", target_commit, e))?;

    let base_tree = repo
        .find_object(base_obj.id(), None)
        .and_then(|o| o.peel_to_commit())
        .and_then(|c| c.tree())
        .map_err(|e| format!("Failed to get base tree: {}", e))?;
    let target_tree = repo
        .find_object(target_obj.id(), None)
        .and_then(|o| o.peel_to_commit())
        .and_then(|c| c.tree())
        .map_err(|e| format!("Failed to get target tree: {}", e))?;

    let diff = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&target_tree), None)
        .map_err(|e| format!("Failed to compute diff: {}", e))?;

    let mut changed_files = Vec::new();

    for delta in diff.deltas() {
        let new_file = delta.new_file();
        if let Some(path) = new_file.path() {
            let path_buf = path.to_path_buf();

            // Skip generated and binary files
            if is_generated(&path_buf) || is_binary_asset(&path_buf) {
                continue;
            }

            let file_type = classify_file(&path_buf, None);

            let changed_file = ChangedFile {
                path: path_buf,
                file_type,
                status: FileStatus::from_git_status(delta.status()),
                additions: 0,
                deletions: 0,
            };

            changed_files.push(changed_file);
        }
    }

    Ok(changed_files)
}

/// Parse a unified diff text into ChangedFile objects.
pub fn parse_diff(repo_path: &Path, diff_text: &str) -> Result<Vec<ChangedFile>, String> {
    let mut changed_files = Vec::new();
    let lines = diff_text.lines().collect::<Vec<_>>();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("diff --git") {
            // Extract path from "diff --git a/path b/path"
            if let Some(a_part) = line.strip_prefix("diff --git a/") {
                let b_start = a_part.find(" b/");
                if let Some(idx) = b_start {
                    let path_str = &a_part[..idx];
                    let path_buf = repo_path.join(path_str);
                    let file_type = classify_file(&path_buf, None);

                    // Count additions/deletions for this file
                    let mut additions = 0;
                    let mut deletions = 0;
                    let mut j = i + 1;
                    while j < lines.len() && !lines[j].starts_with("diff --git") {
                        if lines[j].starts_with("+") && !lines[j].starts_with("+++") {
                            additions += 1;
                        } else if lines[j].starts_with("-") && !lines[j].starts_with("---") {
                            deletions += 1;
                        }
                        j += 1;
                    }

                    changed_files.push(ChangedFile {
                        path: path_buf,
                        file_type,
                        status: FileStatus::Modified,
                        additions,
                        deletions,
                    });

                    i = j - 1; // Continue from next diff header
                }
            }
        }

        i += 1;
    }

    Ok(changed_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_classify_source_file() {
        let path = Path::new("src/main.rs");
        assert_eq!(classify_file(path, None), FileType::Source);
    }

    #[test]
    fn test_classify_test_file() {
        let path = Path::new("tests/integration_test.rs");
        assert_eq!(classify_file(path, None), FileType::Test);
    }

    #[test]
    fn test_classify_config_file() {
        let path = Path::new("Cargo.toml");
        assert_eq!(classify_file(path, None), FileType::Config);
    }

    #[test]
    fn test_classify_doc_file() {
        let path = Path::new("README.md");
        assert_eq!(classify_file(path, None), FileType::Docs);
    }

    #[test]
    fn test_is_generated_lockfile() {
        let path = Path::new("Cargo.lock");
        assert!(is_generated(path));
    }

    #[test]
    fn test_is_binary_asset_image() {
        let path = Path::new("assets/logo.png");
        assert!(is_binary_asset(path));
    }

    #[test]
    fn test_get_changed_files_empty_diff() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        let repo = git2::Repository::init(repo_path).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        // Create initial commit
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let _initial = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Initial commit",
                &repo.find_tree(tree_id).unwrap(),
                &[],
            )
            .unwrap();

        // Get diff between same commit (should be empty)
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let base = target;

        let result = get_changed_files(repo_path, &base.to_string(), &target.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
