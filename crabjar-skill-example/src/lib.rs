/**
 * crabjar-skill-example: Reference skill crate demonstrating the complete
 * flow from manifest → loader → binder → registry.
 */

use crabjar_skills::{SkillLoader, ToolDefinition};
use crabjar_tool_binding::ToolBinder;

/// Example: Create a skill manifest programmatically and bind it to the registry.
pub fn create_example_skill_manifest() -> crabjar_skills::SkillManifest {
    crabjar_skills::SkillManifest {
        name: "example-skill".to_string(),
        version: "0.1.0".to_string(),
        description: "Reference skill demonstrating the complete flow".to_string(),
        author: Some("crabjar-team".to_string()),
        license: Some("MIT".to_string()),
        tools: vec![
            ToolDefinition {
                name: "greet".to_string(),
                description: "Greet someone by name".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                })),
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            },
            ToolDefinition {
                name: "add".to_string(),
                description: "Add two numbers".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["a", "b"]
                })),
                execution_policy: "low_risk".to_string(),
                trust_layer: 3,
            },
        ],
        dependencies: Vec::new(),
        context_requirements: None,
    }
}

/// Example: Discover skills from a directory and bind them to the registry.
pub async fn discover_and_bind(
    skill_dir: &std::path::Path,
    registry: &crabjar_tool_registry::ToolRegistry<'_>,
) -> Result<usize, crabjar_tool_binding::BindingError> {
    let loader = SkillLoader::new();
    let skills = loader.discover_from(skill_dir).map_err(|e| {
        crabjar_tool_binding::BindingError::RegistrationFailed(
            "skill discovery".to_string(),
            e.to_string(),
        )
    })?;

    // Convert Vec to HashMap for bind_all
    let mut skill_map = std::collections::HashMap::new();
    for skill in &skills {
        skill_map.insert(skill.manifest.name.clone(), skill.clone());
    }

    let binder = ToolBinder::new(registry);
    let results = binder.bind_all(&skill_map)?;

    Ok(results.values().sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_example_manifest() {
        let manifest = create_example_skill_manifest();
        assert_eq!(manifest.name, "example-skill");
        assert_eq!(manifest.tools.len(), 2);
        assert_eq!(manifest.tools[0].name, "greet");
        assert_eq!(manifest.tools[1].name, "add");
    }

    #[tokio::test]
    async fn test_discover_and_bind_flow() {
        // Create temp directory with a skill manifest
        let tmp_dir = tempfile::tempdir().unwrap();
        let skill_dir = tmp_dir.path().join("example-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let manifest = create_example_skill_manifest();
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(skill_dir.join("manifest.json"), &manifest_json).unwrap();

        // Create tool registry in temp dir
        let db_path = tmp_dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = crabjar_tool_registry::ToolRegistry::new(&conn);
        registry.init().unwrap();

        // Discover and bind
        let bound = discover_and_bind(tmp_dir.path(), &registry)
            .await
            .expect("should discover and bind");
        assert_eq!(bound, 2);

        // Verify tools are registered
        let all_tools = registry.list_all(10).unwrap();
        assert_eq!(all_tools.len(), 2);
    }
}