//! Decision criteria loading, validation, and library management.
//! Handles JSON schema validation and semantic distinctness checks.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Error type for criteria operations.
#[derive(Debug, thiserror::Error)]
pub enum CriteriaError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("Schema validation errors: {0:?}")]
    Schema(Vec<String>),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for criteria operations.
pub type CriteriaResult<T> = Result<T, CriteriaError>;

impl From<super::decide_backend::BackendError> for CriteriaError {
    fn from(e: super::decide_backend::BackendError) -> Self {
        CriteriaError::Validation(format!("Embedding backend error: {}", e))
    }
}

/// A single decision criterion option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionOption {
    pub id: String,
    pub description: String,
}

/// Decision criteria library definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCriteria {
    pub name: Option<String>,
    #[serde(default = "default_version")]
    pub version: String,
    pub options: Vec<CriterionOption>,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    /// Optional rule-based fast-path matching (JSON string with "rules" array)
    #[serde(default)]
    pub rules: Option<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_threshold() -> f64 {
    0.75
}

/// Metadata about a criteria library for listing.
#[derive(Debug, Clone, Serialize)]
pub struct CriteriaLibraryInfo {
    pub name: String,
    pub path: String,
    pub options: Vec<String>,
    pub version: String,
}

/// Load decision criteria from a file path or inline JSON.
pub fn load_criteria(source: &str) -> CriteriaResult<DecisionCriteria> {
    // Inline JSON is prefixed with '@'
    if source.starts_with('@') {
        let json_str = &source[1..];
        return serde_json::from_str::<DecisionCriteria>(json_str)
            .map_err(CriteriaError::JsonParse);
    }

    // Try as file path
    let path = PathBuf::from(source);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let criteria: DecisionCriteria = serde_json::from_str(&content)?;
        return Ok(criteria);
    }

    // Try discovery paths
    for discovered in discover_criteria(source) {
        let content = fs::read_to_string(&discovered).map_err(|e| CriteriaError::Io(e))?;
        let criteria: DecisionCriteria = serde_json::from_str(&content)?;
        return Ok(criteria);
    }

    Err(CriteriaError::NotFound(format!(
        "Criteria not found at '{}' or in discovery paths",
        source
    )))
}

/// Discover criteria file from standard locations.
fn discover_criteria(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // System install location
    let system_path = PathBuf::from(format!("/opt/crabjar/criteria/{}.json", name));
    if system_path.exists() {
        paths.push(system_path);
    }

    // User override location
    if let Some(home) = dirs::home_dir() {
        let user_path = home.join(format!(".crabjar/criteria/{}.json", name));
        if user_path.exists() {
            paths.push(user_path);
        }
    }

    paths
}

/// List available criteria libraries.
pub fn list_criteria(search_paths: Option<&str>) -> CriteriaResult<Vec<CriteriaLibraryInfo>> {
    let mut search_dirs = Vec::new();

    if let Some(path) = search_paths {
        search_dirs.push(PathBuf::from(path));
    } else {
        // Default search paths
        search_dirs.push(PathBuf::from("/opt/crabjar/criteria"));
        if let Some(home) = dirs::home_dir() {
            search_dirs.push(home.join(".crabjar/criteria"));
        }
    }

    let mut libraries = Vec::new();

    for dir in &search_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            // Try to load and extract metadata
            match load_criteria(path.to_string_lossy().as_ref()) {
                Ok(criteria) => {
                    let name = criteria.name.clone().unwrap_or_else(|| {
                        path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    });
                    let option_ids: Vec<String> =
                        criteria.options.iter().map(|o| o.id.clone()).collect();
                    libraries.push(CriteriaLibraryInfo {
                        name,
                        path: path.to_string_lossy().to_string(),
                        options: option_ids,
                        version: criteria.version,
                    });
                }
                Err(_) => {
                    // Skip invalid files but note them
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    libraries.push(CriteriaLibraryInfo {
                        name: format!("{} (invalid)", stem),
                        path: path.to_string_lossy().to_string(),
                        options: vec![],
                        version: String::new(),
                    });
                }
            }
        }
    }

    Ok(libraries)
}

/// Validate criteria for structural correctness and semantic distinctness.
pub fn validate_criteria(
    path: &str,
    backend: &dyn super::decide_backend::EmbeddingBackend,
) -> CriteriaResult<serde_json::Value> {
    let criteria = load_criteria(path)?;

    // Schema validation
    let mut errors = Vec::new();
    if criteria.options.is_empty() {
        errors.push("No options defined".to_string());
    }
    for option in &criteria.options {
        if option.id.trim().is_empty() {
            errors.push("Option has empty id".to_string());
        }
        if option.description.trim().is_empty() {
            errors.push(format!("Option '{}' has empty description", option.id));
        }
    }

    // Check for duplicate IDs
    let ids: Vec<&str> = criteria.options.iter().map(|o| o.id.as_str()).collect();
    let unique_ids: std::collections::HashSet<&str> = ids.iter().copied().collect();
    if ids.len() != unique_ids.len() {
        errors.push("Duplicate option IDs found".to_string());
    }

    // Semantic distinctness check (embed all options and compare pairwise)
    let mut min_similarity = 1.0;
    let mut most_similar_pair = (String::new(), String::new(), 0.0);

    if criteria.options.len() >= 2 {
        let descriptions: Vec<&str> = criteria
            .options
            .iter()
            .map(|o| o.description.as_str())
            .collect();
        let embeddings = backend.embed_batch(&descriptions)?;

        for i in 0..criteria.options.len() {
            for j in (i + 1)..criteria.options.len() {
                let sim = super::decide_backend::cosine_similarity(&embeddings[i], &embeddings[j]);
                if sim < min_similarity {
                    min_similarity = sim;
                }
                if sim > most_similar_pair.2 {
                    most_similar_pair = (
                        criteria.options[i].id.clone(),
                        criteria.options[j].id.clone(),
                        sim,
                    );
                }
            }
        }

        // Warn if options are too similar (>0.9 cosine similarity)
        if most_similar_pair.2 > 0.9 {
            errors.push(format!(
                "Options '{}' and '{}' are too similar (cosine: {:.3}); consider merging or rewording",
                most_similar_pair.0, most_similar_pair.1, most_similar_pair.2
            ));
        }
    }

    let valid = errors.is_empty();

    Ok(serde_json::json!({
        "success": valid,
        "type": "criteria_validation",
        "file": path,
        "valid": valid,
        "checks": {
            "schema_valid": true,
            "options_count": criteria.options.len(),
            "min_similarity_ok": most_similar_pair.2 <= 0.9,
            "min_similarity": min_similarity,
            "most_similar_pair": {
                "options": [most_similar_pair.0, most_similar_pair.1],
                "similarity": most_similar_pair.2
            }
        },
        "errors": if valid { serde_json::Value::Null } else { json!(errors) }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_inline_criteria() {
        let json = r#"{"name":"test","options":[{"id":"a","description":"option A"},{"id":"b","description":"option B"}]}"#;
        let criteria = load_criteria(&format!("@{}", json)).unwrap();
        assert_eq!(criteria.options.len(), 2);
        assert_eq!(criteria.options[0].id, "a");
    }

    #[test]
    fn test_default_threshold() {
        let json = r#"{"options":[{"id":"a","description":"A"}]}"#;
        let criteria = load_criteria(&format!("@{}", json)).unwrap();
        assert_eq!(criteria.threshold, 0.75);
    }

    #[test]
    fn test_list_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let libs = list_criteria(Some(dir.path().to_string_lossy().as_ref())).unwrap();
        assert_eq!(libs.len(), 0);
    }

    #[test]
    fn test_validate_catches_duplicate_ids() {
        let json = r#"{"options":[{"id":"a","description":"A"},{"id":"a","description":"B"}]}"#;
        let criteria = load_criteria(&format!("@{}", json)).unwrap();
        let ids: Vec<&str> = criteria.options.iter().map(|o| o.id.as_str()).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), 2);
        assert_eq!(unique.len(), 1); // Duplicate detected
    }
}
