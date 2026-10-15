use crate::types::{ChangedFile, FileType, ReviewBundle};
use std::collections::HashMap;
use std::path::Path;

/// Try to determine the crate name from a file path in a Cargo workspace.
fn get_crate_name(path: &Path) -> Option<String> {
    let components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    // Look for Cargo.toml or src/ pattern to identify crate boundary
    for (i, component) in components.iter().enumerate() {
        if i + 1 < components.len() && components[i + 1] == "src" {
            return Some(component.to_string());
        }
    }

    None
}

/// Pair source files with their corresponding test files.
fn pair_source_with_tests(files: &[ChangedFile]) -> HashMap<usize, Vec<usize>> {
    let mut pairs = HashMap::new();
    let n = files.len();

    for i in 0..n {
        if files[i].file_type != FileType::Source {
            continue;
        }

        let source_path = &files[i].path;
        let source_stem = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Look for test files with matching stem
        for j in 0..n {
            if i == j || files[j].file_type != FileType::Test {
                continue;
            }

            let test_path = &files[j].path;
            let test_stem = test_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Match patterns: foo.rs + foo_test.rs, or foo.rs + tests/foo.rs
            if test_stem == source_stem || test_stem == format!("{}_test", source_stem) {
                pairs.entry(i).or_insert_with(Vec::new).push(j);
            }
        }
    }

    pairs
}

/// Create review bundles from changed files.
pub fn create_bundles(files: Vec<ChangedFile>) -> Vec<ReviewBundle> {
    if files.is_empty() {
        return vec![];
    }

    // Pair source files with their tests
    let test_pairs = pair_source_with_tests(&files);

    // Track which files have been bundled
    let mut bundled = vec![false; files.len()];
    let mut bundles = Vec::new();
    let mut bundle_id = 0;

    // First pass: bundle source+test pairs
    for (source_idx, test_indices) in &test_pairs {
        if bundled[*source_idx] {
            continue;
        }

        let mut bundle_files = vec![files[*source_idx].clone()];
        bundled[*source_idx] = true;

        for &test_idx in test_indices {
            if !bundled[test_idx] {
                bundle_files.push(files[test_idx].clone());
                bundled[test_idx] = true;
            }
        }

        let crate_name = get_crate_name(&bundle_files[0].path);
        bundles.push(ReviewBundle {
            id: format!("bundle-{}", bundle_id),
            files: bundle_files,
            crate_name,
        });
        bundle_id += 1;
    }

    // Second pass: bundle remaining files by crate boundary
    let mut current_crate = None;
    let mut current_bundle = Vec::new();

    for (i, file) in files.iter().enumerate() {
        if bundled[i] {
            continue;
        }

        let file_crate = get_crate_name(&file.path);

        // If crate changed or we're at capacity, flush current bundle
        if current_bundle.is_empty() || file_crate == current_crate {
            current_bundle.push(file.clone());
            bundled[i] = true;
            current_crate = file_crate;
        } else {
            // Flush existing bundle
            bundles.push(ReviewBundle {
                id: format!("bundle-{}", bundle_id),
                files: std::mem::take(&mut current_bundle),
                crate_name: current_crate.clone(),
            });
            bundle_id += 1;

            // Start new bundle
            current_bundle.push(file.clone());
            bundled[i] = true;
            current_crate = file_crate;
        }
    }

    // Flush final bundle
    if !current_bundle.is_empty() {
        bundles.push(ReviewBundle {
            id: format!("bundle-{}", bundle_id),
            files: current_bundle,
            crate_name: current_crate,
        });
    }

    bundles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangedFile, FileStatus};
    use std::path::PathBuf;

    fn make_file(path: &str, file_type: FileType) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            file_type,
            status: FileStatus::Modified,
            additions: 10,
            deletions: 5,
        }
    }

    #[test]
    fn test_pair_source_with_tests() {
        let files = vec![
            make_file("src/foo.rs", FileType::Source),
            make_file("tests/foo_test.rs", FileType::Test),
            make_file("src/bar.rs", FileType::Source),
        ];

        let pairs = pair_source_with_tests(&files);
        // foo.rs should be paired with foo_test.rs
        assert!(pairs.contains_key(&0));
        assert_eq!(pairs.get(&0).unwrap().len(), 1);
        // bar.rs has no test file
        assert!(!pairs.contains_key(&2));
    }

    #[test]
    fn test_create_bundles_empty() {
        let bundles = create_bundles(vec![]);
        assert_eq!(bundles.len(), 0);
    }

    #[test]
    fn test_create_bundles_source_test_pairing() {
        let files = vec![
            make_file("src/foo.rs", FileType::Source),
            make_file("tests/foo_test.rs", FileType::Test),
            make_file("src/bar.rs", FileType::Source),
        ];

        let bundles = create_bundles(files);
        // Should have 2 bundles: foo+test, bar
        assert_eq!(bundles.len(), 2);
        assert_eq!(bundles[0].file_count(), 2);
        assert_eq!(bundles[1].file_count(), 1);
    }

    #[test]
    fn test_create_bundles_crate_boundaries() {
        let files = vec![
            make_file("crate-a/src/main.rs", FileType::Source),
            make_file("crate-b/src/main.rs", FileType::Source),
        ];

        let bundles = create_bundles(files);
        // Should have 2 bundles (different crates)
        assert_eq!(bundles.len(), 2);
    }
}
