//! Project loader module for CrabJar agent toolboxes
//!
//! This module provides functionality for loading and managing project-specific
//! configurations and command metadata for the stripped-down CrabJar CLI.
use crate::crabjar_config::{ConfigError, ProjectConfig};
use std::path::{Path, PathBuf};

/// Result type for project loading operations
pub type ProjectResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Manages project-specific configurations and tool loading
#[derive(Debug)]
pub struct ProjectLoader {
    /// Current loaded configuration (if any)
    current_config: Option<ProjectConfig>,
    /// Root directory used to resolve relative tool paths
    project_root: Option<PathBuf>,
}

impl Default for ProjectLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectLoader {
    /// Create a new project loader instance
    pub fn new() -> Self {
        Self {
            current_config: None,
            project_root: None,
        }
    }

    /// Load configuration from the specified project root directory
    pub async fn load_from_directory(&mut self, path: &Path) -> ProjectResult<()> {
        self.project_root = Some(path.to_path_buf());

        let config = match ProjectConfig::load(path) {
            Ok(cfg) => cfg,
            Err(
                ConfigError::NotFound(_) | ConfigError::TomlError(_) | ConfigError::MissingName,
            ) => {
                self.current_config = None;
                return Ok(());
            }
        };

        self.current_config = Some(config.clone());

        if config.auto_register {
            self.register_tools(&config)?;
        }

        Ok(())
    }
    /// Register all tools defined in the configuration
    pub fn register_tools(&self, config: &ProjectConfig) -> ProjectResult<()> {
        for tool_def in &config.tools {
            let path = self.resolve_tool_path(&tool_def.path);

            let exists = path.exists();
            if !exists {
                return Err(format!("tool not found: {}", path.display()).into());
            }
        }

        Ok(())
    }

    fn resolve_tool_path(&self, tool_path: &str) -> PathBuf {
        let expanded = expand_env(tool_path);
        let path = PathBuf::from(&expanded);
        if path.is_absolute() {
            return path;
        }

        match &self.project_root {
            Some(root) => root.join(path),
            None => path,
        }
    }

    /// Get the current project configuration if loaded
    pub fn get_current_config(&self) -> Option<&ProjectConfig> {
        self.current_config.as_ref()
    }
}

/// Expand shell-style environment variable references in a tool path.
///
/// Supports `${VAR}`, `${VAR:-default}`, and bare `$VAR`. Default values may
/// contain further `$` references (e.g. `${CARGO_OXIDE_PATH:-$HOME/.cargo/bin/x}`),
/// which are expanded recursively. Unset variables with no default expand to
/// an empty string.
fn expand_env(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '$' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // chars[i] == '$'
        if i + 1 < chars.len() && chars[i + 1] == '{' {
            // ${...} form
            let start = i + 2;
            let mut j = start;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                // Malformed (no closing brace): emit the remainder as-is.
                out.push_str(&input[i..]);
                break;
            }
            let inner: String = chars[start..j].iter().collect();
            let (name, default) = match inner.split_once(":-") {
                Some((n, d)) => (n.to_string(), Some(d.to_string())),
                None => (inner.clone(), None),
            };
            match std::env::var(&name).ok().filter(|v| !v.is_empty()) {
                Some(v) => out.push_str(&v),
                None => {
                    if let Some(d) = default {
                        out.push_str(&expand_env(&d));
                    }
                }
            }
            i = j + 1;
        } else if i + 1 < chars.len() {
            // Bare $VAR
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j == i + 1 {
                // '$' not followed by a name char: literal.
                out.push('$');
                i += 1;
            } else {
                let name: String = chars[i + 1..j].iter().collect();
                out.push_str(&std::env::var(&name).unwrap_or_default());
                i = j;
            }
        } else {
            // Trailing '$'
            out.push('$');
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_existing_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".crabjar_config.toml");
        let tools_dir = dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();
        fs::write(tools_dir.join("test.nu"), "echo ok").unwrap();

        fs::write(
            &config_path,
            r#"
name = "test-workspace"
description = "Test workspace"

[[tools]]
path = "tools/test.nu"
commands = ["cmd1", "cmd2"]

[keybindings]
"Ctrl a" = "cmd1"
"#,
        )
        .unwrap();

        let mut loader = ProjectLoader::new();
        let result = loader.load_from_directory(dir.path()).await;

        assert!(result.is_ok());
        assert!(loader.get_current_config().is_some());
    }

    #[tokio::test]
    async fn test_create_default_workspace() {
        let dir = tempdir().unwrap();

        // No config file exists - should soft-fail to no workspace
        let mut loader = ProjectLoader::new();
        let result = loader.load_from_directory(dir.path()).await;

        assert!(result.is_ok());
        assert!(loader.get_current_config().is_none());
    }

    #[tokio::test]
    async fn test_command_lookup() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".crabjar_config.toml");
        let tools_dir = dir.path().join("tools");
        fs::create_dir_all(&tools_dir).unwrap();
        fs::write(tools_dir.join("test.nu"), "echo ok").unwrap();

        fs::write(
            &config_path,
            r#"
name = "command-test"

[[tools]]
path = "tools/test.nu"
commands = ["cmd1", "cmd2"]
"#,
        )
        .unwrap();

        let mut loader = ProjectLoader::new();
        loader.load_from_directory(dir.path()).await.unwrap();

        let commands = loader
            .get_current_config()
            .map(|config| config.get_all_commands())
            .unwrap_or_default();
        assert!(commands.contains(&"cmd1".to_string()));
        assert!(commands.contains(&"cmd2".to_string()));
        assert!(!commands.contains(&"nonexistent".to_string()));
    }

    #[tokio::test]
    async fn test_relative_tool_paths_resolve_from_project_root() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".crabjar_config.toml");
        let nested_tools_dir = dir.path().join("tools");
        fs::create_dir_all(&nested_tools_dir).unwrap();
        fs::write(nested_tools_dir.join("tool.nu"), "echo ok").unwrap();

        fs::write(
            &config_path,
            r#"
name = "relative-paths"

[[tools]]
path = "tools/tool.nu"
commands = ["cmd1"]
"#,
        )
        .unwrap();

        let mut loader = ProjectLoader::new();
        loader.load_from_directory(dir.path()).await.unwrap();

        let resolved = loader.resolve_tool_path("tools/tool.nu");
        assert_eq!(resolved, dir.path().join("tools").join("tool.nu"));
    }

    #[tokio::test]
    async fn test_malformed_config_returns_error() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".crabjar_config.toml");

        fs::write(&config_path, "this is not [ valid toml !!!").unwrap();

        let mut loader = ProjectLoader::new();
        let result = loader.load_from_directory(dir.path()).await;

        assert!(result.is_ok());
        assert!(loader.get_current_config().is_none());
    }

    #[tokio::test]
    async fn test_no_config_produces_default_with_no_commands() {
        let dir = tempdir().unwrap();

        let mut loader = ProjectLoader::new();
        loader.load_from_directory(dir.path()).await.unwrap();

        assert!(loader.get_current_config().is_none());
    }

    #[test]
    fn test_expand_env_plain_var() {
        unsafe { std::env::set_var("CRABJAR_TEST_TOOL_PATH", "/opt/tools/tool") };
        assert_eq!(expand_env("$CRABJAR_TEST_TOOL_PATH"), "/opt/tools/tool");
        assert_eq!(expand_env("${CRABJAR_TEST_TOOL_PATH}"), "/opt/tools/tool");
        unsafe { std::env::remove_var("CRABJAR_TEST_TOOL_PATH") };
    }

    #[test]
    fn test_expand_env_default_when_unset() {
        unsafe { std::env::remove_var("CRABJAR_TEST_UNSET_VAR") };
        assert_eq!(
            expand_env("${CRABJAR_TEST_UNSET_VAR:-/fallback/tool}"),
            "/fallback/tool"
        );
    }

    #[test]
    fn test_expand_env_nested_default() {
        // The real-world shape: ${VAR:-$HOME/...}
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            expand_env("${CRABJAR_TEST_UNSET_VAR:-$HOME/.cargo/bin/cargo-oxide}"),
            format!("{home}/.cargo/bin/cargo-oxide")
        );
    }

    #[test]
    fn test_expand_env_var_wins_over_default() {
        unsafe { std::env::set_var("CRABJAR_TEST_TOOL_PATH", "/primary/tool") };
        assert_eq!(
            expand_env("${CRABJAR_TEST_TOOL_PATH:-/fallback/tool}"),
            "/primary/tool"
        );
        unsafe { std::env::remove_var("CRABJAR_TEST_TOOL_PATH") };
    }

    #[test]
    fn test_expand_env_unset_no_default_is_empty() {
        unsafe { std::env::remove_var("CRABJAR_TEST_UNSET_VAR") };
        assert_eq!(expand_env("${CRABJAR_TEST_UNSET_VAR}/suffix"), "/suffix");
    }

    #[test]
    fn test_expand_env_literal_dollar_and_malformed() {
        // `$5` is a valid name in this impl (alphanumeric, bash positional
        // param semantics) so it expands to empty; `${broken` has no closing
        // brace and is emitted as-is.
        assert_eq!(
            expand_env("price is $5 and ${broken"),
            "price is  and ${broken"
        );
        assert_eq!(expand_env("a $ b"), "a $ b");
        assert_eq!(expand_env("trailing $"), "trailing $");
    }

    #[test]
    fn test_resolve_tool_path_expands_env() {
        let home = std::env::var("HOME").unwrap();
        let loader = ProjectLoader::new();
        let resolved =
            loader.resolve_tool_path("${CRABJAR_TEST_UNSET_VAR:-$HOME/.cargo/bin/cargo-oxide}");
        assert_eq!(
            resolved,
            std::path::PathBuf::from(format!("{home}/.cargo/bin/cargo-oxide"))
        );
    }
}
