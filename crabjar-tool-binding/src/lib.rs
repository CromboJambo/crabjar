//! Tool binding layer — registers skill tools with the tool registry.
//!
//! Bridges `crabjar-skills` loaded skills to the `tool_registry` so that
//! tools declared in skill manifests become callable by agents through
//! execution policy enforcement.

use crabjar_skills::LoadedSkill;
use crabjar_tool_registry::ToolRegistry;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Error types for tool binding operations.
#[derive(Debug, thiserror::Error)]
pub enum BindingError {
    #[error("failed to register tool '{0}': {1}")]
    RegistrationFailed(String, String),
    #[error("invalid execution policy '{0}' for tool '{1}'")]
    InvalidPolicy(String, String),
}

/// The tool binder — binds skill tools to the tool registry.
pub struct ToolBinder<'a> {
    registry: &'a ToolRegistry<'a>,
}

impl<'a> ToolBinder<'a> {
    /// Create a new tool binder for the given registry.
    #[must_use]
    pub fn new(registry: &'a ToolRegistry<'a>) -> Self {
        Self { registry }
    }

    /// Bind all tools from a single loaded skill to the registry.
    ///
    /// Validates execution policies and skips duplicate tool names (first wins).
    /// Returns the number of tools successfully bound.
    pub fn bind_skill(&self, skill: &LoadedSkill) -> Result<usize, BindingError> {
        let mut bound = 0;

        for tool in skill.tools() {
            // Validate execution policy
            if !matches!(
                tool.execution_policy.as_str(),
                "low_risk" | "medium_risk" | "high_risk"
            ) {
                return Err(BindingError::InvalidPolicy(
                    tool.execution_policy.clone(),
                    tool.name.clone(),
                ));
            }

            // Serialize parameters schema
            let schema = match &tool.parameters {
                Some(params) => serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };

            // Build metadata with skill origin
            let metadata = serde_json::json!({
                "skill": skill.manifest.name,
                "version": skill.manifest.version,
                "path": skill.path.display().to_string()
            })
            .to_string();

            // Check if tool already registered (first wins)
            match self.registry.query_tool(&tool.name) {
                Ok(Some(existing)) => {
                    if existing.metadata != metadata {
                        debug!(
                            tool = %tool.name,
                            "duplicate tool name, first registration wins"
                        );
                    }
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(tool = %tool.name, error = %e, "failed to query existing tool");
                    continue;
                }
            }

            // Register the tool
            match self.registry.register_tool(
                &tool.name,
                "skill",
                &tool.description,
                &schema,
                &tool.execution_policy,
                tool.trust_layer,
                0.8, // Default confidence for skill-provided tools
                &metadata,
            ) {
                Ok(id) => {
                    debug!(
                        tool = %tool.name,
                        id = %id,
                        policy = %tool.execution_policy,
                        "bound tool from skill"
                    );
                    bound += 1;
                }
                Err(e) => {
                    return Err(BindingError::RegistrationFailed(
                        tool.name.clone(),
                        e.to_string(),
                    ));
                }
            }
        }

        debug!(
            skill = %skill.manifest.name,
            bound = bound,
            "bound tools from skill"
        );
        Ok(bound)
    }

    /// Bind all tools from multiple loaded skills.
    ///
    /// Returns a map of skill name → number of tools bound.
    pub fn bind_all(
        &self,
        skills: &HashMap<String, LoadedSkill>,
    ) -> Result<HashMap<String, usize>, BindingError> {
        let mut results = HashMap::new();

        for (name, skill) in skills {
            let bound = self.bind_skill(skill)?;
            results.insert(name.clone(), bound);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabjar_skills::{SkillLoader, SkillManifest, ToolDefinition};
    use std::fs::{create_dir_all, write};
    use tempfile::tempdir;

    fn create_test_skill(dir: &std::path::Path, name: &str) -> LoadedSkill {
        let skill_dir = dir.join(name);
        create_dir_all(&skill_dir).unwrap();

        let manifest = SkillManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: format!("Test skill {}", name),
            author: Some("test".to_string()),
            license: None,
            tools: vec![
                ToolDefinition {
                    name: format!("{}_tool1", name),
                    description: "Test tool 1".to_string(),
                    parameters: None,
                    execution_policy: "low_risk".to_string(),
                    trust_layer: 3,
                },
                ToolDefinition {
                    name: format!("{}_tool2", name),
                    description: "Test tool 2".to_string(),
                    parameters: Some(serde_json::json!({"type": "object"})),
                    execution_policy: "medium_risk".to_string(),
                    trust_layer: 2,
                },
            ],
            dependencies: Vec::new(),
            context_requirements: None,
        };

        let manifest_json = serde_json::to_string(&manifest).unwrap();
        write(skill_dir.join("manifest.json"), manifest_json).unwrap();

        LoadedSkill {
            manifest,
            path: skill_dir,
        }
    }

    #[test]
    fn test_bind_skill_registers_tools() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let skill = create_test_skill(dir.path(), "test-skill");
        let binder = ToolBinder::new(&registry);
        let bound = binder.bind_skill(&skill).unwrap();

        assert_eq!(bound, 2);

        // Verify tools are registered
        let all_tools = registry.list_all(10).unwrap();
        assert_eq!(all_tools.len(), 2);
        assert!(all_tools.iter().any(|t| t.name == "test-skill_tool1"));
        assert!(all_tools.iter().any(|t| t.name == "test-skill_tool2"));
    }

    #[test]
    fn test_bind_skill_execution_policy() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        let skill = create_test_skill(dir.path(), "policy-test");
        let binder = ToolBinder::new(&registry);
        binder.bind_skill(&skill).unwrap();

        // Verify execution policies were preserved
        let tool1 = registry.query_tool("policy-test_tool1").unwrap().unwrap();
        assert_eq!(tool1.execution_policy, "low_risk");

        let tool2 = registry.query_tool("policy-test_tool2").unwrap().unwrap();
        assert_eq!(tool2.execution_policy, "medium_risk");
    }

    #[test]
    fn test_bind_skill_duplicate_names() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        // Create two skills with the same tool name
        let skill1 = LoadedSkill {
            manifest: SkillManifest {
                name: "skill-a".to_string(),
                version: "0.1.0".to_string(),
                description: String::new(),
                author: None,
                license: None,
                tools: vec![ToolDefinition {
                    name: "shared_tool".to_string(),
                    description: "From skill A".to_string(),
                    parameters: None,
                    execution_policy: "low_risk".to_string(),
                    trust_layer: 3,
                }],
                dependencies: Vec::new(),
                context_requirements: None,
            },
            path: dir.path().join("skill-a"),
        };

        let skill2 = LoadedSkill {
            manifest: SkillManifest {
                name: "skill-b".to_string(),
                version: "0.1.0".to_string(),
                description: String::new(),
                author: None,
                license: None,
                tools: vec![ToolDefinition {
                    name: "shared_tool".to_string(),
                    description: "From skill B".to_string(),
                    parameters: None,
                    execution_policy: "low_risk".to_string(),
                    trust_layer: 3,
                }],
                dependencies: Vec::new(),
                context_requirements: None,
            },
            path: dir.path().join("skill-b"),
        };

        let binder = ToolBinder::new(&registry);
        let bound1 = binder.bind_skill(&skill1).unwrap();
        let bound2 = binder.bind_skill(&skill2).unwrap();

        // First skill binds the tool, second skips it (duplicate)
        assert_eq!(bound1, 1);
        assert_eq!(bound2, 0);

        // Only one registration exists
        let all_tools = registry.list_all(10).unwrap();
        assert_eq!(all_tools.len(), 1);
    }

    #[test]
    fn test_bind_all_skills() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        // Create multiple skills via loader
        let tools_dir = dir.path().join("tools");
        create_dir_all(&tools_dir).unwrap();
        create_test_skill(&tools_dir, "multi-skill-1");
        create_test_skill(&tools_dir, "multi-skill-2");

        let loader = SkillLoader::new();
        let skills = loader.discover_from(&tools_dir).unwrap();

        // Convert Vec to HashMap for bind_all
        let mut skill_map = HashMap::new();
        for skill in &skills {
            skill_map.insert(skill.manifest.name.clone(), skill.clone());
        }

        let binder = ToolBinder::new(&registry);
        let results = binder.bind_all(&skill_map).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get("multi-skill-1").unwrap(), &2);
        assert_eq!(results.get("multi-skill-2").unwrap(), &2);

        // Total of 4 tools registered
        let all_tools = registry.list_all(10).unwrap();
        assert_eq!(all_tools.len(), 4);
    }

    #[test]
    fn test_bind_skill_invalid_policy() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tool_registry.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let registry = ToolRegistry::new(&conn);
        registry.init().unwrap();

        // Create skill with invalid execution policy
        let skill = LoadedSkill {
            manifest: SkillManifest {
                name: "bad-policy".to_string(),
                version: "0.1.0".to_string(),
                description: String::new(),
                author: None,
                license: None,
                tools: vec![ToolDefinition {
                    name: "test_tool".to_string(),
                    description: "Test".to_string(),
                    parameters: None,
                    execution_policy: "invalid_policy".to_string(),
                    trust_layer: 3,
                }],
                dependencies: Vec::new(),
                context_requirements: None,
            },
            path: dir.path().join("bad-policy"),
        };

        let binder = ToolBinder::new(&registry);
        let result = binder.bind_skill(&skill);

        assert!(result.is_err());
        match result.unwrap_err() {
            BindingError::InvalidPolicy(policy, name) => {
                assert_eq!(policy, "invalid_policy");
                assert_eq!(name, "test_tool");
            }
            e => panic!("Expected InvalidPolicy error, got {:?}", e),
        }
    }
}
