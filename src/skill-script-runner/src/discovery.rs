use anyhow::Result;

use crate::discover_skills;
use crate::find_scripts;

/// Discover all skill directories and their bundled scripts.
pub fn discover_all(
    project_root: &std::path::Path,
    home_dir: &std::path::Path,
) -> Result<Vec<(std::path::PathBuf, Vec<std::path::PathBuf>)>> {
    let skill_dirs = discover_skills(project_root, home_dir)?;

    let mut results = Vec::new();
    for skill_dir in skill_dirs {
        let scripts = find_scripts(&skill_dir)?;
        results.push((skill_dir, scripts));
    }

    Ok(results)
}

/// Filter scripts by skill name.
pub fn filter_by_skill(
    discoveries: &[(std::path::PathBuf, Vec<std::path::PathBuf>)],
    skill_name: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let mut found_scripts = Vec::new();
    for (skill_dir, scripts) in discoveries {
        if skill_dir.file_name().map(|n| n.to_string_lossy())
            == Some(std::borrow::Cow::Borrowed(skill_name))
        {
            found_scripts.extend(scripts.clone());
        }
    }

    Ok(found_scripts)
}

/// Discover skills and return them as skillsets (grouped by category or directory).
///
/// This is a convenience wrapper around `discover_all` that converts the flat
/// skill list into grouped skillsets using the skillset conversion logic.
pub fn discover_as_skillsets(
    project_root: &std::path::Path,
    home_dir: &std::path::Path,
) -> Result<Vec<skillset::SkillSet>> {
    let discoveries = discover_all(project_root, home_dir)?;

    // Convert discoveries to skill records
    let mut records = Vec::new();
    for (skill_dir, scripts) in &discoveries {
        // Determine source based on path
        let source = if home_dir.ancestors().any(|a| skill_dir.starts_with(a)) {
            skillset::SkillSource::User
        } else {
            skillset::SkillSource::Project
        };

        // Parse SKILL.md frontmatter
        let skill_md = skill_dir.join("SKILL.md");
        let content = std::fs::read_to_string(&skill_md).ok();

        let mut id = String::new();
        let mut name = String::new();
        let mut description = String::new();
        let mut category: Option<String> = None;

        if let Some(c) = content {
            // Parse YAML frontmatter (simple parser for --- delimited frontmatter)
            if let Some((_, body)) = c.split_once("---\n")
                && let Some((fm, _)) = body.split_once("---\n")
            {
                for line in fm.lines() {
                    if let Some((k, v)) = line.split_once(": ") {
                        match k.trim() {
                            "id" => id = v.trim().to_string(),
                            "name" => name = v.trim().to_string(),
                            "category" => category = Some(v.trim().to_string()),
                            _ => {}
                        }
                    }
                }
            }
            // Extract first non-empty line as description fallback
            for line in c.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    description = trimmed.to_string();
                    break;
                }
            }
        }

        // Defaults
        if id.is_empty() {
            id = skill_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
        }
        if name.is_empty() {
            name = id.clone();
        }
        if description.is_empty() {
            description = format!("Skill: {}", name);
        }

        records.push(skillset::SkillRecord {
            id,
            name,
            description,
            category,
            source,
            skill_path: skill_md.to_string_lossy().to_string(),
            skill_dir: skill_dir.to_string_lossy().to_string(),
            has_scripts: !scripts.is_empty(),
        });
    }

    // Convert to skillsets
    skillset::convert_to_skillsets(&records).map_err(|e| anyhow::anyhow!("{}", e))
}

// Re-export skillset types for convenience
pub use skill_reference_store::skillset;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_all_returns_empty_when_no_skills() {
        let project_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();
        let result = discover_all(project_dir.path(), home_dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn discover_all_finds_skills_with_scripts() {
        let project_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();

        let skills_dir = project_dir.path().join(".agents/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\n",
        )
        .unwrap();

        let scripts_dir = skill_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("run.sh"), "#!/bin/bash\necho hello\n").unwrap();

        let result = discover_all(project_dir.path(), home_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, skill_dir);
        assert_eq!(result[0].1.len(), 1);
    }

    #[test]
    fn discover_all_skills_without_scripts() {
        let project_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();

        let skills_dir = project_dir.path().join(".agents/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("no-scripts");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: no-scripts\ndescription: test\n---\n",
        )
        .unwrap();

        let result = discover_all(project_dir.path(), home_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.len(), 0);
    }

    #[test]
    fn filter_by_skill_finds_matching() {
        let project_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();

        let skills_dir = project_dir.path().join(".agents/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\n",
        )
        .unwrap();

        let scripts_dir = skill_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("a.sh"), "").unwrap();
        std::fs::write(scripts_dir.join("b.sh"), "").unwrap();

        let discoveries = discover_all(project_dir.path(), home_dir.path()).unwrap();
        let scripts = filter_by_skill(&discoveries, "my-skill").unwrap();
        assert_eq!(scripts.len(), 2);
    }

    #[test]
    fn filter_by_skill_no_match() {
        let project_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();

        let skills_dir = project_dir.path().join(".agents/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let skill_dir = skills_dir.join("other-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: other-skill\ndescription: test\n---\n",
        )
        .unwrap();

        let discoveries = discover_all(project_dir.path(), home_dir.path()).unwrap();
        let scripts = filter_by_skill(&discoveries, "nonexistent").unwrap();
        assert!(scripts.is_empty());
    }
}
