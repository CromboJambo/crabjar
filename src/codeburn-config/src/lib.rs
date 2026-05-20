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

    pub fn default() -> Self {
        Self::new()
    }

    pub fn load(path: &Path) -> Result<Self, CodeBurnConfigError> {
        if !path.exists() {
            return Err(CodeBurnConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| CodeBurnConfigError::ParseError { reason: e.to_string() })?;

        toml::from_str(&content)
            .map_err(|e| CodeBurnConfigError::ParseError { reason: e.to_string() })
    }

    pub fn plan_usage(&self, _name: &str) -> Result<String, CodeBurnConfigError> {
        Ok("default".to_string())
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
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert!(config.workspace.is_none());
        assert_eq!(config.currency, "USD");
    }
}
