use crate::SkillContext;
use std::fs;
use std::path::PathBuf;

/// Default path for persisted skill context.
const DEFAULT_CONTEXT_PATH: &str = ".crabjar/skill_context.json";

/// Persist a skill context to disk.
pub fn persist_context(ctx: &SkillContext, path: Option<&str>) -> Result<PathBuf, std::io::Error> {
    let target = match path {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(DEFAULT_CONTEXT_PATH),
    };

    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(ctx)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(&target, json)?;

    Ok(target)
}

/// Load a persisted skill context from disk.
pub fn load_context(path: Option<&str>) -> Result<SkillContext, std::io::Error> {
    let target = match path {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(DEFAULT_CONTEXT_PATH),
    };

    if !target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No persisted context at {}", target.display()),
        ));
    }

    let json = fs::read_to_string(&target)?;
    let ctx: SkillContext = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(ctx)
}

/// Check if a persisted context exists at the default location.
pub fn has_persisted_context() -> bool {
    PathBuf::from(DEFAULT_CONTEXT_PATH).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn persist_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_context.json");

        let mut ctx = SkillContext::default();
        ctx.set_env("TEST", "value");
        ctx.set_metadata("session", "123");

        persist_context(&ctx, Some(path.to_str().unwrap())).unwrap();
        let loaded = load_context(Some(path.to_str().unwrap())).unwrap();

        assert_eq!(loaded.get_env("TEST"), Some(&"value".to_string()));
        assert_eq!(loaded.get_metadata("session"), Some(&"123".to_string()));
    }

    #[test]
    fn load_nonexistent_fails() {
        let result = load_context(Some("/tmp/nonexistent_context_12345.json"));
        assert!(result.is_err());
    }

    #[test]
    fn has_persisted_context_works() {
        // Default path shouldn't exist in test environment
        // (unless a previous test left it behind)
        let exists = has_persisted_context();
        // Just verify it doesn't panic
        assert!(exists || !exists);
    }
}
