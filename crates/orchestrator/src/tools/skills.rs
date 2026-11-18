//! Skill management tools — list, view, and manage skills.
//!
//! Provides `skills_list`, `skill_view`, and `skill_manage` for agent-driven
//! procedural memory management. Skills are discovered from the filesystem
//! using the crabjar-skills crate's SkillLoader.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Handle a skills_list tool call.
pub async fn handle_skills_list(args: &[String]) -> Result<String, String> {
    let category = parse_optional_category(args)?;

    let loader = crabjar_skills::SkillLoader::new();
    let skills = loader.discover().map_err(|e| format!("Failed to discover skills: {}", e))?;

    let mut skill_list: Vec<Value> = skills
        .into_iter()
        .filter(|(_, skill)| {
            category
                .as_ref()
                .map_or(true, |cat| skill.manifest.name.starts_with(cat))
        })
        .map(|(name, skill)| {
            json!({
                "name": name,
                "description": skill.manifest.description,
                "version": skill.manifest.version,
                "tools": skill.tools().iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            })
        })
        .collect();

    skill_list.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    let result = json!({
        "skills": skill_list,
        "count": skill_list.len(),
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

/// Handle a skill_view tool call.
pub async fn handle_skill_view(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: skill_view requires a skill name".to_string());
    }

    let input: Value = serde_json::from_str(&args[0]).map_err(|e| format!("JSON parse error: {}", e))?;
    let name = input["name"].as_str().ok_or("Missing required field: name")?;
    let file_path = input["file_path"].as_str();

    let loader = crabjar_skills::SkillLoader::new();
    let skills = loader.discover().map_err(|e| format!("Failed to discover skills: {}", e))?;

    let skill = skills
        .get(name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    // If file_path is specified, read that file from the skill directory
    if let Some(fp) = file_path {
        let path = skill.path.join(fp);
        if !path.exists() {
            return Err(format!("File '{}' not found in skill '{}'", fp, name));
        }
        let content = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
        let result = json!({
            "name": name,
            "file_path": fp,
            "content": content,
        });
        return Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?);
    }

    // Otherwise, return the full manifest
    let result = json!({
        "name": name,
        "version": skill.manifest.version,
        "description": skill.manifest.description,
        "author": skill.manifest.author,
        "license": skill.manifest.license,
        "tools": skill.tools(),
        "dependencies": skill.manifest.dependencies,
        "path": skill.path.display().to_string(),
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

/// Handle a skill_manage tool call.
pub async fn handle_skill_manage(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("Error: skill_manage requires JSON arguments".to_string());
    }

    let input: Value = serde_json::from_str(&args[0]).map_err(|e| format!("JSON parse error: {}", e))?;
    let operations = input["operations"]
        .as_array()
        .ok_or("Missing required field: operations")?;

    if operations.is_empty() {
        return Err("Error: at least one operation is required".to_string());
    }

    let mut results = Vec::new();

    for op in operations {
        let action = op["action"].as_str().ok_or("Missing action field")?;

        match action {
            "create" => handle_skill_create(op, &mut results),
            "patch" => handle_skill_patch(op, &mut results),
            "delete" => handle_skill_delete(op, &mut results),
            _ => return Err(format!("Unknown skill operation: {}", action)),
        }?;
    }

    let result = json!({
        "success": true,
        "operations_completed": results.len(),
        "results": results,
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

fn handle_skill_create(op: &Value, results: &mut Vec<Value>) -> Result<(), String> {
    let name = op["name"].as_str().ok_or("Missing skill name")?;
    let content = op["content"]
        .as_str()
        .ok_or("Missing skill content (manifest JSON)")?;

    // Validate the manifest
    let manifest: crabjar_skills::SkillManifest =
        serde_json::from_str(content).map_err(|e| format!("Invalid manifest JSON: {}", e))?;

    let errors = manifest.validate();
    if !errors.is_empty() {
        return Err(format!("Invalid manifest: {:?}", errors));
    }

    // Create the skill directory in the default location
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let skills_dir = home.join(".corust-agent/skills");
    let skill_dir = skills_dir.join(name);

    if skill_dir.exists() {
        return Err(format!("Skill '{}' already exists", name));
    }

    std::fs::create_dir_all(&skill_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
    std::fs::write(skill_dir.join("manifest.json"), content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    results.push(json!({
        "action": "create",
        "name": name,
        "path": skill_dir.display().to_string(),
    }));

    Ok(())
}

fn handle_skill_patch(op: &Value, results: &mut Vec<Value>) -> Result<(), String> {
    let name = op["name"].as_str().ok_or("Missing skill name")?;
    let old_string = op["old_string"]
        .as_str()
        .ok_or("Missing old_string field")?;
    let new_string = op["new_string"].as_str().unwrap_or("");

    // Find the skill
    let loader = crabjar_skills::SkillLoader::new();
    let skills = loader.discover().map_err(|e| format!("Failed to discover skills: {}", e))?;
    let skill = skills
        .get(name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    // Read the manifest
    let manifest_path = skill.path.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;

    if !content.contains(old_string) {
        return Err(format!("old_string not found in skill '{}'", name));
    }

    let new_content = content.replace(old_string, new_string);
    std::fs::write(&manifest_path, &new_content).map_err(|e| e.to_string())?;

    results.push(json!({
        "action": "patch",
        "name": name,
        "file": "manifest.json",
    }));

    Ok(())
}

fn handle_skill_delete(op: &Value, results: &mut Vec<Value>) -> Result<(), String> {
    let name = op["name"].as_str().ok_or("Missing skill name")?;

    // Find the skill
    let loader = crabjar_skills::SkillLoader::new();
    let skills = loader.discover().map_err(|e| format!("Failed to discover skills: {}", e))?;
    let skill = skills
        .get(name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    std::fs::remove_dir_all(&skill.path).map_err(|e| format!("Failed to delete skill: {}", e))?;

    results.push(json!({
        "action": "delete",
        "name": name,
    }));

    Ok(())
}

fn parse_optional_category(args: &[String]) -> Result<Option<String>, String> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(None);
    }
    let input: Value = serde_json::from_str(&args[0]).map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(input["category"].as_str().map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skills_list_no_args() {
        let result = handle_skills_list(&[]).await;
        assert!(result.is_ok());
        let value: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(value["count"].as_u64().unwrap_or(0) >= 0);
    }

    #[tokio::test]
    async fn test_skill_view_missing_name() {
        let result = handle_skill_view(&[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_skill_manage_missing_operations() {
        let result = handle_skill_manage(&[r#"{}"#.to_string()]).await;
        assert!(result.is_err());
    }
}
