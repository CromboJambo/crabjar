//! Skill loader — discovers and loads skills from the filesystem.
//!
//! Scans directories for skill manifests (manifest.json) and loads them
//! into in-memory Skill objects ready for tool binding.

use crate::error::SkillError;
use crate::manifest::{SkillManifest, ToolDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

/// A loaded skill with its manifest and filesystem location.
#[derive(Debug, Clone)]
pub struct LoadedSkill {
    /// The parsed skill manifest.
    pub manifest: SkillManifest,
    /// Absolute path to the skill directory on disk.
    pub path: PathBuf,
}

impl LoadedSkill {
    /// Get all tool definitions from this skill.
    #[must_use]
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.manifest.tools
    }
}

/// The skill loader — discovers and loads skills from directories.
pub struct SkillLoader {
    /// Directories to scan for skills.
    search_paths: Vec<PathBuf>,
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillLoader {
    /// Create a new skill loader with default search paths.
    ///
    /// Default paths: `.agents/skills`, `~/.corust-agent/skills`, `~/.agents/skills`
    #[must_use]
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // Project-level skills directory
        if let Ok(cwd) = std::env::current_dir() {
            search_paths.push(cwd.join(".agents/skills"));
        }

        // User-level skill directories
        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(&home).join(".corust-agent/skills"));
            search_paths.push(PathBuf::from(&home).join(".agents/skills"));
        }

        Self { search_paths }
    }

    /// Add a custom skill directory to scan.
    #[must_use]
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.search_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Discover all skills from configured search paths.
    ///
    /// Returns a map of skill name → `LoadedSkill`. If multiple directories
    /// contain the same skill name, the first one wins (priority order).
    pub fn discover(&self) -> Result<HashMap<String, LoadedSkill>, SkillError> {
        let mut skills = HashMap::new();

        for search_path in &self.search_paths {
            if !search_path.is_dir() {
                debug!(path = %search_path.display(), "skill search path not found");
                continue;
            }

            debug!(path = %search_path.display(), "scanning skill directory");

            let entries = std::fs::read_dir(search_path)?;

            for entry in entries.flatten() {
                let skill_dir = entry.path();

                if !skill_dir.is_dir() {
                    continue;
                }

                // Check for manifest.json
                let manifest_path = skill_dir.join("manifest.json");
                if !manifest_path.exists() {
                    continue;
                }

                // Read and parse the manifest
                let content = std::fs::read_to_string(&manifest_path)?;
                match SkillManifest::parse(&content) {
                    Ok(manifest) => {
                        // Validate the manifest
                        let errors = manifest.validate();
                        if !errors.is_empty() {
                            debug!(
                                skill = %manifest.name,
                                path = %skill_dir.display(),
                                "invalid skill manifest: {:?}",
                                errors
                            );
                            continue;
                        }

                        // Skip duplicates (first wins)
                        if skills.contains_key(&manifest.name) {
                            debug!(
                                skill = %manifest.name,
                                "duplicate skill found, skipping"
                            );
                            continue;
                        }

                        let loaded = LoadedSkill {
                            manifest,
                            path: skill_dir,
                        };

                        skills.insert(loaded.manifest.name.clone(), loaded);
                    }
                    Err(e) => {
                        debug!(
                            path = %skill_dir.display(),
                            error = %e,
                            "failed to parse skill manifest, skipping"
                        );
                    }
                }
            }
        }

        debug!(count = skills.len(), "skills discovered");
        Ok(skills)
    }

    /// Discover skills from a single directory.
    pub fn discover_from<P: AsRef<Path>>(&self, path: P) -> Result<Vec<LoadedSkill>, SkillError> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(SkillError::DirectoryNotFound(path.to_path_buf()));
        }

        let mut skills = Vec::new();
        let entries = std::fs::read_dir(path)?;

        for entry in entries.flatten() {
            let skill_dir = entry.path();

            if !skill_dir.is_dir() {
                continue;
            }

            let manifest_path = skill_dir.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&manifest_path)?;
            match SkillManifest::parse(&content) {
                Ok(manifest) => {
                    skills.push(LoadedSkill {
                        manifest,
                        path: skill_dir,
                    });
                }
                Err(e) => {
                    debug!(
                        path = %skill_dir.display(),
                        error = %e,
                        "failed to parse skill manifest, skipping"
                    );
                }
            }
        }

        Ok(skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use tempfile::tempdir;

    fn create_test_skill(dir: &Path, name: &str, tools: Vec<ToolDefinition>) -> PathBuf {
        let skill_dir = dir.join(name);
        create_dir_all(&skill_dir).unwrap();

        let manifest = SkillManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: format!("Test skill {}", name),
            author: Some("test".to_string()),
            license: None,
            tools,
            dependencies: Vec::new(),
            context_requirements: None,
        };

        let manifest_json = serde_json::to_string(&manifest).unwrap();
        write(skill_dir.join("manifest.json"), manifest_json).unwrap();

        skill_dir
    }

    #[test]
    fn test_discover_no_skills() {
        let loader = SkillLoader::new();
        let skills = loader.discover().unwrap();
        // May find real skills in the environment, so just check it doesn't error
        assert!(skills.iter().all(|(name, _)| !name.is_empty()));
    }

    #[test]
    fn test_discover_from_directory() {
        let temp = tempdir().unwrap();
        let tools_dir = temp.path().join("tools");
        create_dir_all(&tools_dir).unwrap();

        create_test_skill(
            &tools_dir,
            "test-skill-1",
            vec![ToolDefinition {
                name: "tool1".to_string(),
                description: String::new(),
                parameters: None,
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            }],
        );

        create_test_skill(&tools_dir, "test-skill-2", Vec::new());

        let loader = SkillLoader::new();
        let skills = loader.discover_from(&tools_dir).unwrap();

        assert_eq!(skills.len(), 2);
        assert!(skills.iter().any(|s| s.manifest.name == "test-skill-1"));
        assert!(skills.iter().any(|s| s.manifest.name == "test-skill-2"));
    }

    #[test]
    fn test_discover_duplicate_names() {
        let temp = tempdir().unwrap();
        let tools_dir = temp.path().join("tools");
        create_dir_all(&tools_dir).unwrap();

        // Two skills with the same name in different subdirectories
        let dir1 = create_test_skill(
            &tools_dir,
            "dup-skill",
            vec![ToolDefinition {
                name: "tool1".to_string(),
                description: String::new(),
                parameters: None,
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            }],
        );

        // Create a second directory with the same skill name (should be skipped)
        let dir2 = tools_dir.join("dup-skill-copy");
        std::fs::rename(&dir1, &dir2).unwrap();

        // Now create the original again
        create_test_skill(
            &tools_dir,
            "dup-skill",
            vec![ToolDefinition {
                name: "tool2".to_string(),
                description: String::new(),
                parameters: None,
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            }],
        );

        let loader = SkillLoader::new();
        let skills = loader.discover_from(&tools_dir).unwrap();

        // Should find both directories (they have different names now)
        assert!(skills.iter().any(|s| s.manifest.name == "dup-skill"));
    }

    #[test]
    fn test_invalid_manifest_skipped() {
        let temp = tempdir().unwrap();
        let tools_dir = temp.path().join("tools");
        create_dir_all(&tools_dir).unwrap();

        // Create a skill with invalid manifest (not valid JSON)
        let bad_dir = tools_dir.join("bad-skill");
        create_dir_all(&bad_dir).unwrap();
        write(bad_dir.join("manifest.json"), "not valid json").unwrap();

        // Create a good skill alongside it
        create_test_skill(
            &tools_dir,
            "good-skill",
            vec![ToolDefinition {
                name: "tool1".to_string(),
                description: String::new(),
                parameters: None,
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            }],
        );

        let loader = SkillLoader::new();
        let skills = loader.discover_from(&tools_dir).unwrap();

        // Should find only the good skill
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].manifest.name, "good-skill");
    }
}
