use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Execution context provided to loaded skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    /// Current working directory for skill execution.
    pub cwd: PathBuf,

    /// Environment variables available to the skill.
    pub env: HashMap<String, String>,

    /// Session state (arbitrary JSON-serializable data).
    #[serde(default)]
    pub session_state: serde_json::Value,

    /// Skill-specific configuration overrides.
    #[serde(default)]
    pub config_overrides: HashMap<String, serde_json::Value>,

    /// User-provided metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for SkillContext {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let env = std::env::vars().collect();

        Self {
            cwd,
            env,
            session_state: serde_json::Value::Null,
            config_overrides: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl SkillContext {
    /// Create a new context with default values from the current process environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current working directory.
    pub fn set_cwd<P: Into<PathBuf>>(&mut self, path: P) {
        self.cwd = path.into();
    }

    /// Get an environment variable.
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Set an environment variable.
    pub fn set_env<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.env.insert(key.into(), value.into());
    }

    /// Remove an environment variable.
    pub fn remove_env<K: AsRef<str>>(&mut self, key: K) {
        self.env.remove(key.as_ref());
    }

    /// Set the entire session state.
    pub fn set_session_state(&mut self, state: serde_json::Value) {
        self.session_state = state;
    }

    /// Get the current session state.
    pub fn get_session_state(&self) -> &serde_json::Value {
        &self.session_state
    }

    /// Set a skill-specific configuration override.
    pub fn set_config_override<K: Into<String>>(&mut self, key: K, value: serde_json::Value) {
        self.config_overrides.insert(key.into(), value);
    }

    /// Get a skill-specific configuration override.
    pub fn get_config_override(&self, key: &str) -> Option<&serde_json::Value> {
        self.config_overrides.get(key)
    }

    /// Set metadata.
    pub fn set_metadata<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Serialize the context to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a context from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_has_cwd() {
        let ctx = SkillContext::default();
        assert!(ctx.cwd.exists());
    }

    #[test]
    fn env_variables_are_accessible() {
        let mut ctx = SkillContext::default();
        ctx.set_env("TEST_VAR", "test_value");
        assert_eq!(ctx.get_env("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn session_state_roundtrip() {
        let mut ctx = SkillContext::default();
        let state = serde_json::json!({"counter": 42});
        ctx.set_session_state(state);
        assert_eq!(ctx.get_session_state()["counter"], 42);
    }

    #[test]
    fn config_overrides() {
        let mut ctx = SkillContext::default();
        ctx.set_config_override("timeout", serde_json::json!(30));
        assert_eq!(
            ctx.get_config_override("timeout"),
            Some(&serde_json::json!(30))
        );
    }

    #[test]
    fn serialization_roundtrip() {
        let mut ctx = SkillContext::default();
        ctx.set_env("KEY", "value");
        ctx.set_metadata("author", "test");

        let json = ctx.to_json().unwrap();
        let restored = SkillContext::from_json(&json).unwrap();

        assert_eq!(restored.get_env("KEY"), Some(&"value".to_string()));
        assert_eq!(restored.get_metadata("author"), Some(&"test".to_string()));
    }
}
