//! crabjar-config
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),
    #[error("Failed to parse TOML: {0}")]
    TomlError(String),
    #[error("Workspace name not specified")]
    MissingName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub path: String,
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(rename = "name")]
    pub workspace_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_store_path: Option<String>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default, rename = "keybindings")]
    pub keybindings: HashMap<String, String>,
    #[serde(default)]
    pub tool_execution_enabled: bool,
    #[serde(default = "default_true", skip_serializing_if = "bool::clone")]
    pub auto_register: bool,
}

fn default_true() -> bool {
    true
}

impl ProjectConfig {
    pub fn load(project_root: &Path) -> Result<Self, ConfigError> {
        let config_path = project_root.join(".crabjar_config.toml");
        if !config_path.exists() {
            return Err(ConfigError::NotFound(config_path));
        }
        let content =
            fs::read_to_string(&config_path).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        Self::parse_from_str(&content)
    }

    pub fn parse_from_str(toml_str: &str) -> Result<Self, ConfigError> {
        let config: ProjectConfig =
            toml::from_str(toml_str).map_err(|e| ConfigError::TomlError(e.to_string()))?;
        if config.workspace_name.is_empty() {
            return Err(ConfigError::MissingName);
        }
        Ok(config)
    }

    pub fn get_all_commands(&self) -> Vec<String> {
        self.tools.iter().flat_map(|t| t.commands.clone()).collect()
    }

    pub fn has_command(&self, command: &str) -> bool {
        self.tools
            .iter()
            .flat_map(|t| t.commands.iter())
            .any(|c| c == command)
    }

    pub fn get_keybinding_action(&self, key: &str) -> Option<String> {
        self.keybindings.get(key).cloned()
    }
}

#[derive(Debug, Default)]
pub struct ProjectConfigBuilder {
    name: String,
    description: Option<String>,
    knowledge_store_path: Option<String>,
    tools: Vec<ToolDefinition>,
    keybindings: HashMap<String, String>,
    tool_execution_enabled: bool,
    auto_register: bool,
}

impl ProjectConfigBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            auto_register: true,
            ..Default::default()
        }
    }
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    pub fn add_tool(mut self, path: impl Into<String>, cmds: Vec<String>) -> Self {
        self.tools.push(ToolDefinition {
            path: path.into(),
            commands: cmds,
        });
        self
    }
    pub fn knowledge_store_path(mut self, path: impl Into<String>) -> Self {
        self.knowledge_store_path = Some(path.into());
        self
    }
    pub fn keybinding(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.keybindings.insert(key.into(), value.into());
        self
    }
    pub fn no_auto_register(mut self) -> Self {
        self.auto_register = false;
        self
    }
    pub fn build(self) -> Result<ProjectConfig, ConfigError> {
        if self.name.is_empty() {
            return Err(ConfigError::MissingName);
        }
        Ok(ProjectConfig {
            workspace_name: self.name,
            description: self.description,
            knowledge_store_path: self.knowledge_store_path,
            tools: self.tools,
            keybindings: self.keybindings,
            tool_execution_enabled: self.tool_execution_enabled,
            auto_register: self.auto_register,
        })
    }
}

pub fn generate_template(name: &str) -> String {
    format!(
        "name = \"{}\"\ndescription = \"Custom CrabJar workspace for {}\"\n\nauto_register = true\n\n[[tools]]\npath = \"data-transformations.nu\"\ncommands = [\"load-data\", \"transform-pipeline\"]\n\n[keybindings]\n\"Ctrl a\" = \"load-data\"\n",
        name, name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse() {
        let toml_str = r#"
name = "test"

[[tools]]
path = "t.nu"
commands = ["cmd"]
"#;
        assert!(ProjectConfig::parse_from_str(toml_str).is_ok());
    }

    #[test]
    fn parse_missing_name_returns_error() {
        let toml_str = r#"
description = "no name here"
"#;
        let result = ProjectConfig::parse_from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_name_returns_error() {
        let toml_str = r#"
name = ""
"#;
        let result = ProjectConfig::parse_from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn parse_preserves_description() {
        let toml_str = r#"
name = "test"
description = "A test workspace"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert_eq!(config.description, Some("A test workspace".to_string()));
    }

    #[test]
    fn parse_no_description_is_none() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.description.is_none());
    }

    #[test]
    fn parse_tools_with_multiple_entries() {
        let toml_str = r#"
name = "test"

[[tools]]
path = "tool1.nu"
commands = ["cmd1", "cmd2"]

[[tools]]
path = "tool2.nu"
commands = ["cmd3"]
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert_eq!(config.tools.len(), 2);
        assert_eq!(config.tools[0].path, "tool1.nu");
        assert_eq!(config.tools[0].commands, vec!["cmd1", "cmd2"]);
        assert_eq!(config.tools[1].path, "tool2.nu");
    }

    #[test]
    fn parse_keybindings() {
        let toml_str = r#"
name = "test"

[keybindings]
"Ctrl a" = "load-data"
"Ctrl b" = "transform"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert_eq!(config.keybindings.len(), 2);
        assert_eq!(config.keybindings.get("Ctrl a"), Some(&"load-data".to_string()));
        assert_eq!(config.keybindings.get("Ctrl b"), Some(&"transform".to_string()));
    }

    #[test]
    fn parse_no_keybindings_is_empty() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.keybindings.is_empty());
    }

    #[test]
    fn parse_auto_register_defaults_true() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.auto_register);
    }

    #[test]
    fn parse_auto_register_can_be_false() {
        let toml_str = r#"
name = "test"
auto_register = false
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(!config.auto_register);
    }

    #[test]
    fn parse_tool_execution_defaults_false() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(!config.tool_execution_enabled);
    }

    #[test]
    fn parse_tool_execution_can_be_true() {
        let toml_str = r#"
name = "test"
tool_execution_enabled = true
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.tool_execution_enabled);
    }

    #[test]
    fn get_all_commands_flattens_tools() {
        let toml_str = r#"
name = "test"

[[tools]]
path = "t1.nu"
commands = ["cmd1", "cmd2"]

[[tools]]
path = "t2.nu"
commands = ["cmd3"]
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        let commands = config.get_all_commands();
        assert_eq!(commands, vec!["cmd1", "cmd2", "cmd3"]);
    }

    #[test]
    fn get_all_commands_empty_when_no_tools() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        let commands = config.get_all_commands();
        assert!(commands.is_empty());
    }

    #[test]
    fn has_command_finds_existing() {
        let toml_str = r#"
name = "test"

[[tools]]
path = "t.nu"
commands = ["deploy", "build"]
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.has_command("deploy"));
        assert!(config.has_command("build"));
    }

    #[test]
    fn has_command_returns_false_for_missing() {
        let toml_str = r#"
name = "test"

[[tools]]
path = "t.nu"
commands = ["deploy"]
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(!config.has_command("destroy"));
    }

    #[test]
    fn get_keybinding_action_returns_value() {
        let toml_str = r#"
name = "test"

[keybindings]
"Ctrl a" = "load-data"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert_eq!(config.get_keybinding_action("Ctrl a"), Some("load-data".to_string()));
    }

    #[test]
    fn get_keybinding_action_returns_none_for_missing() {
        let toml_str = r#"
name = "test"
"#;
        let config = ProjectConfig::parse_from_str(toml_str).unwrap();
        assert!(config.get_keybinding_action("Ctrl a").is_none());
    }

    #[test]
    fn builder_creates_minimal_config() {
        let config = ProjectConfigBuilder::new("test-workspace").build().unwrap();
        assert_eq!(config.workspace_name, "test-workspace");
        assert!(config.description.is_none());
        assert!(config.knowledge_store_path.is_none());
        assert!(config.tools.is_empty());
        assert!(config.keybindings.is_empty());
        assert!(config.auto_register);
    }

    #[test]
    fn builder_with_description() {
        let config = ProjectConfigBuilder::new("test")
            .description("A test workspace")
            .build()
            .unwrap();
        assert_eq!(config.description, Some("A test workspace".to_string()));
    }

    #[test]
    fn builder_with_tool() {
        let config = ProjectConfigBuilder::new("test")
            .add_tool("deploy.nu", vec!["deploy".to_string(), "rollback".to_string()])
            .build()
            .unwrap();
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].path, "deploy.nu");
        assert_eq!(config.tools[0].commands, vec!["deploy".to_string(), "rollback".to_string()]);
    }

    #[test]
    fn builder_with_knowledge_store_path() {
        let config = ProjectConfigBuilder::new("test")
            .knowledge_store_path("/custom/path.db")
            .build()
            .unwrap();
        assert_eq!(config.knowledge_store_path, Some("/custom/path.db".to_string()));
    }

    #[test]
    fn builder_with_keybinding() {
        let config = ProjectConfigBuilder::new("test")
            .keybinding("Ctrl s", "save")
            .build()
            .unwrap();
        assert_eq!(config.keybindings.get("Ctrl s"), Some(&"save".to_string()));
    }

    #[test]
    fn builder_no_auto_register() {
        let config = ProjectConfigBuilder::new("test")
            .no_auto_register()
            .build()
            .unwrap();
        assert!(!config.auto_register);
    }

    #[test]
    fn builder_empty_name_returns_error() {
        let result = ProjectConfigBuilder::new("").build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_full_config() {
        let config = ProjectConfigBuilder::new("full-workspace")
            .description("Full test")
            .add_tool("tool1.nu", vec!["cmd1".to_string()])
            .add_tool("tool2.nu", vec!["cmd2".to_string(), "cmd3".to_string()])
            .knowledge_store_path("/tmp/knowledge.db")
            .keybinding("Ctrl a", "action-a")
            .keybinding("Ctrl b", "action-b")
            .no_auto_register()
            .build()
            .unwrap();

        assert_eq!(config.workspace_name, "full-workspace");
        assert_eq!(config.description, Some("Full test".to_string()));
        assert_eq!(config.knowledge_store_path, Some("/tmp/knowledge.db".to_string()));
        assert_eq!(config.tools.len(), 2);
        assert_eq!(config.keybindings.len(), 2);
        assert!(!config.auto_register);
        assert!(!config.tool_execution_enabled);
    }

    #[test]
    fn generate_template_contains_name() {
        let template = generate_template("my-workspace");
        assert!(template.contains("my-workspace"));
        assert!(template.contains("name = \"my-workspace\""));
        assert!(template.contains("auto_register = true"));
        assert!(template.contains("[[tools]]"));
        assert!(template.contains("[keybindings]"));
    }

    #[test]
    fn config_error_not_found() {
        let path = std::path::PathBuf::from("/nonexistent/.crabjar_config.toml");
        let err = ConfigError::NotFound(path.clone());
        assert!(err.to_string().contains("/nonexistent/.crabjar_config.toml"));
    }

    #[test]
    fn config_error_toml_error() {
        let err = ConfigError::TomlError("parse failed".to_string());
        assert!(err.to_string().contains("parse failed"));
    }

    #[test]
    fn config_error_missing_name() {
        let err = ConfigError::MissingName;
        assert!(err.to_string().contains("Workspace name not specified"));
    }

    #[test]
    fn config_error_debug_format() {
        let err = ConfigError::NotFound(std::path::PathBuf::from("/test"));
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }
}
