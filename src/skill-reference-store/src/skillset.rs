//! Skillset grouping: convert individual skill records into grouped skillsets.
//!
//! Skills in the crabjar ecosystem are discovered as individual SKILL.md directories.
//! This module provides the conversion from flat skill lists into categorized skillsets,
//! enabling batch loading, scoped execution, and structured tool registration.
//!
//! ## Mapping Rules
//!
//! 1. **Category grouping**: Skills with the same `category` frontmatter field are grouped.
//! 2. **Directory grouping** (fallback): Skills without a category are grouped by their
//!    parent directory name (e.g., `.agents/skills/<dir>` → skillset name = `<dir>`).
//! 3. **Source grouping**: Skills are tagged with their source (`user` or `project`)
//!    for scope-aware loading.
//! 4. **Overlap handling**: A skill belongs to exactly one skillset (first matching rule).
//! 5. **Missing references**: Skills with no valid SKILL.md are excluded with a warning.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during skillset conversion.
#[derive(Debug, Error)]
pub enum SkillSetError {
    #[error("no skills provided to convert")]
    NoSkills,

    #[error("invalid skill metadata for '{name}': {reason}")]
    InvalidMetadata { name: String, reason: String },

    #[error("conflicting categories for skill '{name}': {categories:?}")]
    ConflictingCategories {
        name: String,
        categories: Vec<String>,
    },

    #[error("duplicate skillset name '{name}'")]
    DuplicateName { name: String },
}

/// A single skill record as discovered from the filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    /// Unique identifier (directory name or SKILL.md frontmatter `id`).
    pub id: String,
    /// Display name (from SKILL.md frontmatter `name`).
    pub name: String,
    /// One-line description (from SKILL.md frontmatter `description`).
    pub description: String,
    /// Category extracted from frontmatter `category` field.
    /// If None, the skill will be grouped by directory name.
    pub category: Option<String>,
    /// Source of the skill: user-level or project-level.
    pub source: SkillSource,
    /// Path to the SKILL.md file.
    pub skill_path: String,
    /// Path to the skill directory.
    pub skill_dir: String,
    /// Whether the skill has bundled scripts.
    pub has_scripts: bool,
}

/// Where a skill was discovered from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSource {
    /// User-level: ~/.corust-agent/skills or ~/.agents/skills
    User,
    /// Project-level: <project>/.agents/skills
    Project,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::User => write!(f, "user"),
            SkillSource::Project => write!(f, "project"),
        }
    }
}

/// A grouped collection of skills — a "skillset".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSet {
    /// Unique identifier for the skillset.
    pub id: String,
    /// Human-readable name (derived from category or directory).
    pub name: String,
    /// The category this skillset represents.
    pub category: Option<String>,
    /// Source scope (all members share the same source).
    pub source: SkillSource,
    /// List of skill IDs belonging to this skillset.
    pub member_ids: Vec<String>,
    /// Total number of scripts across all member skills.
    pub total_scripts: usize,
    /// Description of the skillset (best-effort from first member).
    pub description: String,
}

impl SkillSet {
    /// Create a new skillset from its components.
    pub fn new(
        id: String,
        name: String,
        category: Option<String>,
        source: SkillSource,
        member_ids: Vec<String>,
        total_scripts: usize,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            category,
            source,
            member_ids,
            total_scripts,
            description,
        }
    }

    /// Check if this skillset contains the given skill id.
    pub fn contains(&self, skill_id: &str) -> bool {
        self.member_ids.contains(&skill_id.to_string())
    }

    /// Get the number of member skills.
    pub fn member_count(&self) -> usize {
        self.member_ids.len()
    }
}

/// Convert a flat list of skill records into grouped skillsets.
///
/// ## Grouping rules
/// - Primary: group by `category` field (case-insensitive).
/// - Fallback: group by directory name (parent of the skill dir).
/// - All skills in a group must share the same `source`.
///
/// ## Returns
/// A list of `SkillSet` instances, or an error if grouping fails.
pub fn convert_to_skillsets(skills: &[SkillRecord]) -> Result<Vec<SkillSet>, SkillSetError> {
    if skills.is_empty() {
        return Err(SkillSetError::NoSkills);
    }

    // Group skills by their grouping key (category or directory).
    // Key format: "category:devops" or "directory:creative"
    let mut groups: std::collections::HashMap<String, Vec<&SkillRecord>> =
        std::collections::HashMap::new();

    for skill in skills {
        let key = match &skill.category {
            Some(cat) => format!("category:{}", cat.to_lowercase()),
            None => {
                // Fallback: use the parent directory name of the skill dir
                let dir_name = std::path::Path::new(&skill.skill_dir)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                format!("directory:{}", dir_name)
            }
        };

        // Check source consistency within group
        if let Some(group) = groups.get_mut(&key) {
            // All members must share the same source
            let source_match = group
                .iter()
                .all(|s: &&SkillRecord| s.source == skill.source);
            if !source_match {
                return Err(SkillSetError::InvalidMetadata {
                    name: skill.name.clone(),
                    reason: format!(
                        "mixed sources in group '{}': skill is '{}', group has '{}'",
                        key, skill.source, group[0].source
                    ),
                });
            }
        }
        groups.entry(key).or_default().push(skill);
    }

    // Convert groups to skillsets
    let mut skillsets = Vec::new();

    for (key, members) in groups {
        let source = members[0].source.clone();
        let member_ids: Vec<String> = members.iter().map(|s| s.id.clone()).collect();
        let total_scripts: usize = members
            .iter()
            .map(|s| if s.has_scripts { 1 } else { 0 })
            .sum();

        // Derive skillset name from the key
        let parts: Vec<&str> = key.splitn(2, ':').collect();
        let group_type = parts.first().copied().unwrap_or("unknown");
        let group_name = parts.get(1).copied().unwrap_or("unknown");

        let set_id = format!("{}-{}", group_type, group_name);
        let set_name = match group_type {
            "category" => group_name.to_string(),
            "directory" => format!("Directory: {}", group_name),
            _ => group_name.to_string(),
        };

        // Extract category from key if applicable
        let category = if group_type == "category" {
            Some(group_name.to_string())
        } else {
            None
        };

        // Best-effort description: first member's description
        let description = members
            .first()
            .map(|s| s.description.clone())
            .unwrap_or_default();

        skillsets.push(SkillSet::new(
            set_id,
            set_name,
            category,
            source,
            member_ids,
            total_scripts,
            description,
        ));
    }

    // Sort by number of members (largest first), then by name
    skillsets.sort_by(|a, b| {
        b.member_ids
            .len()
            .cmp(&a.member_ids.len())
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(skillsets)
}

/// Convert skillsets back to a flat list of skill IDs.
/// Useful for round-trip verification.
pub fn skillset_member_ids(skillsets: &[SkillSet]) -> Vec<String> {
    let mut ids = Vec::new();
    for set in skillsets {
        ids.extend(set.member_ids.iter().cloned());
    }
    ids
}

/// Get the set of all categories across a list of skills.
pub fn extract_categories(skills: &[SkillRecord]) -> Vec<String> {
    let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();
    for skill in skills {
        if let Some(cat) = &skill.category {
            categories.insert(cat.to_lowercase());
        }
    }
    let mut result: Vec<String> = categories.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(
        id: &str,
        name: &str,
        category: Option<&str>,
        source: SkillSource,
        skill_dir: &str,
        has_scripts: bool,
    ) -> SkillRecord {
        SkillRecord {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("Description for {}", name),
            category: category.map(|s| s.to_string()),
            source,
            skill_path: format!("/skills/{}/SKILL.md", id),
            skill_dir: skill_dir.to_string(),
            has_scripts,
        }
    }

    // --- convert_to_skillsets tests ---

    #[test]
    fn convert_empty_list_errors() {
        let result = convert_to_skillsets(&[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            SkillSetError::NoSkills => {}
            other => panic!("expected NoSkills, got {:?}", other),
        }
    }

    #[test]
    fn convert_single_skill_creates_single_set() {
        let skills = vec![make_skill(
            "skill-1",
            "Test Skill",
            Some("devops"),
            SkillSource::User,
            "/home/user/.agents/skills/test-skill",
            false,
        )];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "devops");
        assert_eq!(sets[0].category, Some("devops".to_string()));
        assert_eq!(sets[0].member_count(), 1);
        assert_eq!(sets[0].source, SkillSource::User);
    }

    #[test]
    fn convert_groups_by_category() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/git-helper",
                true,
            ),
            make_skill(
                "skill-2",
                "Docker Manager",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/docker-manager",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].member_count(), 2);
        assert!(sets[0].contains("skill-1"));
        assert!(sets[0].contains("skill-2"));
    }

    #[test]
    fn convert_groups_by_directory_fallback() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                None, // no category — falls back to directory
                SkillSource::User,
                "/home/user/.agents/skills/creative-tools",
                false,
            ),
            make_skill(
                "skill-2",
                "Image Gen",
                None, // no category — falls back to directory
                SkillSource::User,
                "/home/user/.agents/skills/creative-tools",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].member_count(), 2);
        assert_eq!(sets[0].name, "Directory: creative-tools");
        assert_eq!(sets[0].category, None);
    }

    #[test]
    fn convert_creates_multiple_sets_for_different_categories() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/git-helper",
                false,
            ),
            make_skill(
                "skill-2",
                "Image Gen",
                Some("creative"),
                SkillSource::User,
                "/home/user/.agents/skills/image-gen",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 2);

        // Verify both categories are present
        let cat_names: Vec<&str> = sets.iter().map(|s| s.name.as_str()).collect();
        assert!(cat_names.contains(&"creative"));
        assert!(cat_names.contains(&"devops"));
    }

    #[test]
    fn convert_case_insensitive_category_grouping() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Dev Skill",
                Some("DevOps"), // capitalized
                SkillSource::User,
                "/home/user/.agents/skills/dev-skill",
                false,
            ),
            make_skill(
                "skill-2",
                "Ops Skill",
                Some("devops"), // lowercase
                SkillSource::User,
                "/home/user/.agents/skills/ops-skill",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].member_count(), 2);
    }

    #[test]
    fn convert_mixed_category_and_directory() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/git-helper",
                false,
            ),
            make_skill(
                "skill-2",
                "Image Gen",
                Some("creative"),
                SkillSource::User,
                "/home/user/.agents/skills/image-gen",
                false,
            ),
            make_skill(
                "skill-3",
                "No Category Skill",
                None,
                SkillSource::User,
                "/home/user/.agents/skills/misc-tools",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets.len(), 3);
    }

    #[test]
    fn convert_mixed_sources_errors() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Dev Skill",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/dev-skill",
                false,
            ),
            make_skill(
                "skill-2",
                "Project Dev",
                Some("devops"),
                SkillSource::Project,
                "/home/project/.agents/skills/dev-skill",
                false,
            ),
        ];

        let result = convert_to_skillsets(&skills);
        assert!(result.is_err());
        match result.unwrap_err() {
            SkillSetError::InvalidMetadata { .. } => {}
            other => panic!("expected InvalidMetadata, got {:?}", other),
        }
    }

    #[test]
    fn convert_tracks_script_count() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/git-helper",
                true, // has scripts
            ),
            make_skill(
                "skill-2",
                "Docker Manager",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/docker-manager",
                true, // has scripts
            ),
            make_skill(
                "skill-3",
                "Lint Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/lint-helper",
                false, // no scripts
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert_eq!(sets[0].total_scripts, 2);
    }

    #[test]
    fn convert_sorts_by_member_count_descending() {
        let skills = vec![
            make_skill(
                "s1",
                "A",
                Some("creative"),
                SkillSource::User,
                "/d/s1",
                false,
            ),
            make_skill("s2", "B", Some("devops"), SkillSource::User, "/d/s2", false),
            make_skill("s3", "C", Some("devops"), SkillSource::User, "/d/s3", false),
            make_skill("s4", "D", Some("devops"), SkillSource::User, "/d/s4", false),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        // devops (3 members) should come before creative (1 member)
        assert_eq!(sets[0].member_count(), 3);
        assert_eq!(sets[1].member_count(), 1);
    }

    #[test]
    fn convert_skillset_contains_works() {
        let skills = vec![
            make_skill(
                "skill-1",
                "Git Helper",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/git-helper",
                false,
            ),
            make_skill(
                "skill-2",
                "Docker Manager",
                Some("devops"),
                SkillSource::User,
                "/home/user/.agents/skills/docker-manager",
                false,
            ),
        ];

        let sets = convert_to_skillsets(&skills).unwrap();
        assert!(sets[0].contains("skill-1"));
        assert!(sets[0].contains("skill-2"));
        assert!(!sets[0].contains("nonexistent"));
    }

    // --- extract_categories tests ---

    #[test]
    fn extract_categories_returns_sorted_unique() {
        let skills = vec![
            make_skill(
                "s1",
                "A",
                Some("creative"),
                SkillSource::User,
                "/d/s1",
                false,
            ),
            make_skill("s2", "B", Some("devops"), SkillSource::User, "/d/s2", false),
            make_skill(
                "s3",
                "C",
                Some("creative"),
                SkillSource::User,
                "/d/s3",
                false,
            ),
        ];

        let cats = extract_categories(&skills);
        assert_eq!(cats, vec!["creative", "devops"]);
    }

    #[test]
    fn extract_categories_handles_no_categories() {
        let skills = vec![
            make_skill("s1", "A", None, SkillSource::User, "/d/s1", false),
            make_skill("s2", "B", None, SkillSource::User, "/d/s2", false),
        ];

        let cats = extract_categories(&skills);
        assert!(cats.is_empty());
    }

    // --- skillset_member_ids tests ---

    #[test]
    fn skillset_member_ids_collects_all() {
        let sets = vec![
            SkillSet::new(
                "cat-1".to_string(),
                "Set 1".to_string(),
                Some("devops".to_string()),
                SkillSource::User,
                vec!["s1".to_string(), "s2".to_string()],
                1,
                "desc".to_string(),
            ),
            SkillSet::new(
                "cat-2".to_string(),
                "Set 2".to_string(),
                Some("creative".to_string()),
                SkillSource::User,
                vec!["s3".to_string()],
                0,
                "desc".to_string(),
            ),
        ];

        let ids = skillset_member_ids(&sets);
        assert_eq!(ids, vec!["s1", "s2", "s3"]);
    }

    #[test]
    fn skillset_member_ids_empty() {
        let ids = skillset_member_ids(&[]);
        assert!(ids.is_empty());
    }

    // --- SkillSource display tests ---

    #[test]
    fn skill_source_display_user() {
        assert_eq!(format!("{}", SkillSource::User), "user");
    }

    #[test]
    fn skill_source_display_project() {
        assert_eq!(format!("{}", SkillSource::Project), "project");
    }
}
