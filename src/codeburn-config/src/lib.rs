#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn codeburn_config_load_valid_file_works() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
workspace = "test-workspace"
currency = "EUR"
plan = "standard"
[[model_aliases]]
"claude-3-opus" = "claude_opus"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert_eq!(config.workspace, Some("test-workspace".to_string()));
        assert_eq!(config.currency, "EUR");
        assert_eq!(config.plan, Some("standard".to_string()));
    }

    #[test]
    fn codeburn_config_load_malformed_file_errors() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            "not valid toml",
        ).unwrap();
        let result = CodeBurnConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("malformed"));
    }

    #[test]
    fn codeburn_config_load_empty_name_works() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
currency = "USD"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert!(config.workspace.is_none());
    }

    #[test]
    fn codeburn_config_load_with_aliases_works() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
currency = "USD"
[[model_aliases]]
"claude-3-sonnet" = "claude_sonnet"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert!(config.model_aliases.contains_key("claude-3-sonnet"));
    }

    #[test]
    fn codeburn_config_load_with_empty_aliases_works() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
currency = "USD"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert!(config.model_aliases.is_empty());
    }

    #[test]
    fn codeburn_config_load_with_large_aliases_works() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
currency = "USD"
[[model_aliases]]
"claude-3-opus" = "claude_opus"
"claude-3-sonnet" = "claude_sonnet"
"claude-3-haiku" = "claude_haiku"
"claude-4" = "claude_4"
"gpt-4o" = "gpt_4o"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        assert!(config.model_aliases.len() >= 5);
    }

    #[test]
    fn codeburn_config_plan_usage_returns_default() {
        let config = CodeBurnConfig::new();
        let result = config.plan_usage("standard").unwrap();
        assert_eq!(result["plan"], "standard");
        assert_eq!(result["usage"], 0.0);
        assert_eq!(result["remaining"], 0.0);
    }

    #[test]
    fn codeburn_config_plan_usage_for_custom_plan() {
        let config = CodeBurnConfig::new();
        let result = config.plan_usage("custom-plan").unwrap();
        assert_eq!(result["plan"], "custom-plan");
    }

    #[test]
    fn codeburn_config_clone_works() {
        let config = CodeBurnConfig::new();
        let cloned = config.clone();
        assert_eq!(cloned.currency, config.currency);
    }

    #[test]
    fn codeburn_config_serialize_works() {
        let config = CodeBurnConfig::new();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("USD"));
    }

    #[test]
    fn codeburn_config_deserialize_works() {
        let serialized = serde_json::to_string(&CodeBurnConfig::new()).unwrap();
        let deserialized: CodeBurnConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.currency, "USD");
    }

    #[test]
    fn codeburn_config_serialize_with_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".codeburn_config.toml"),
            r#"
workspace = "test"
currency = "USD"
"#,
        ).unwrap();
        let config = CodeBurnConfig::load(dir.path()).unwrap();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("test"));
    }

    #[test]
    fn codeburn_config_deserialize_with_workspace() {
        let serialized = serde_json::to_string(&CodeBurnConfig {
            workspace: Some("test".to_string()),
            currency: "USD".to_string(),
            plan: None,
            model_aliases: BTreeMap::new(),
        }).unwrap();
        let deserialized: CodeBurnConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.workspace, Some("test".to_string()));
    }

    #[test]
    fn codeburn_config_deserialize_with_aliases() {
        let serialized = serde_json::to_string(&CodeBurnConfig {
            workspace: None,
            currency: "USD".to_string(),
            plan: None,
            model_aliases: {
                let mut m = BTreeMap::new();
                m.insert("claude-3-opus", "claude_opus");
                m
            },
        }).unwrap();
        let deserialized: CodeBurnConfig = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.model_aliases.contains_key("claude-3-opus"));
    }
}
