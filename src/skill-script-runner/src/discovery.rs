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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_skills;
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
