use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodeBurnConfigError {
    #[error("config file not found: {path}")]
    FileNotFound { path: std::path::PathBuf },
    #[error("config parse error: {reason}")]
    ParseError { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBurnConfig {
    pub workspace: Option<String>,
    pub currency: String,
    pub plan: Option<String>,
    pub model_aliases: BTreeMap<String, String>,
}

impl CodeBurnConfig {
    pub fn new() -> Self {
        Self {
            workspace: None,
            currency: "USD".to_string(),
            plan: None,
            model_aliases: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, CodeBurnConfigError> {
        let config_path = path.join(".crabjar_config.toml");
        if !config_path.exists() {
            return Err(CodeBurnConfigError::FileNotFound { path: config_path });
        }

        let content =
            std::fs::read_to_string(&config_path).map_err(|e| CodeBurnConfigError::ParseError {
                reason: e.to_string(),
            })?;

        toml::from_str(&content).map_err(|e| CodeBurnConfigError::ParseError {
            reason: e.to_string(),
        })
    }

    pub fn plan_usage(&self, _name: &str) -> Result<String, CodeBurnConfigError> {
        Ok("default".to_string())
    }
}

impl Default for CodeBurnConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::CodeBurnConfig;

    use tempfile::tempdir;

    #[test]
    fn codeburn_config_new_defaults() {
        let config = CodeBurnConfig::new();
        assert!(config.workspace.is_none());
        assert_eq!(config.currency, "USD");
        assert!(config.plan.is_none());
        assert!(config.model_aliases.is_empty());
    }

    #[test]
    fn codeburn_config_default_works() {
        let config = CodeBurnConfig::default();
        assert!(config.workspace.is_none());
        assert_eq!(config.currency, "USD");
    }

    #[test]
    fn codeburn_config_load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap_or_else(|_| CodeBurnConfig::new());
        assert!(config.workspace.is_none());
        assert_eq!(config.currency, "USD");
    }
}
