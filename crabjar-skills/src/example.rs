use crate::manifest::{SkillManifest, ToolDefinition};

/// Example: Create a skill manifest programmatically and bind it to the registry.
///
/// This demonstrates the complete flow from manifest → loader → binder → registry.
pub fn create_example_skill_manifest() -> SkillManifest {
    SkillManifest {
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
}
