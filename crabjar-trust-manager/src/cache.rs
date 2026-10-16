//! Version-aware skill cache in ~/.crabjar/skill-cache/.
//! Supports caching, retrieval, cleanup of stale entries (>30 days).

use crate::error::TrustManagerError;
use crate::models::{SkillCacheEntry, TrustTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Cached skill metadata stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    name: String,
    version: String,
    source: String,
    install_path: String,
    download_url: Option<String>,
    checksum: Option<String>,
    trust_tier: String,
    installed_at: DateTime<Utc>,
    last_used: DateTime<Utc>,
}

/// Manages the skill cache directory and metadata.
pub struct SkillCache {
    cache_dir: PathBuf,
    entries: HashMap<String, SkillCacheEntry>,
}

impl Default for SkillCache {
    fn default() -> Self {
        Self::new().expect("failed to initialize skill cache")
    }
}

impl SkillCache {
    /// Create a new skill cache manager.
    pub fn new() -> Result<Self, TrustManagerError> {
        let home = dirs::home_dir().ok_or_else(|| {
            TrustManagerError::CacheDirError("could not determine home directory".to_string())
        })?;

        let cache_dir = home.join(".crabjar").join("skill-cache");
        fs::create_dir_all(&cache_dir)?;

        let mut entries = HashMap::new();
        Self::load_metadata(&cache_dir, &mut entries)?;

        info!(cache_dir = %cache_dir.display(), "initialized skill cache");

        Ok(Self { cache_dir, entries })
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Cache a downloaded skill with its metadata.
    pub fn cache_skill(
        &mut self,
        name: &str,
        version: &str,
        source: &str,
        install_path: PathBuf,
        download_url: Option<String>,
        checksum: Option<String>,
        trust_tier: TrustTier,
    ) -> Result<SkillCacheEntry, TrustManagerError> {
        let entry = SkillCacheEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: source.to_string(),
            install_path: install_path.to_string_lossy().to_string(),
            download_url,
            checksum,
            trust_tier,
            installed_at: Utc::now(),
            last_used: Utc::now(),
        };

        // Write metadata to disk
        let meta = CacheMetadata {
            name: entry.name.clone(),
            version: entry.version.clone(),
            source: entry.source.clone(),
            install_path: entry.install_path.clone(),
            download_url: entry.download_url.clone(),
            checksum: entry.checksum.clone(),
            trust_tier: format!("{}", entry.trust_tier),
            installed_at: entry.installed_at,
            last_used: entry.last_used,
        };

        let meta_path = self.cache_dir.join(format!("{}.json", name));
        fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

        info!(
            skill = %name,
            version = %version,
            "cached skill"
        );

        self.entries.insert(name.to_string(), entry.clone());
        Ok(entry)
    }

    /// Check if a skill is cached.
    pub fn is_cached(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get cached skill entry by name.
    pub fn get(&self, name: &str) -> Option<&SkillCacheEntry> {
        self.entries.get(name)
    }

    /// Update last_used timestamp for a cached skill.
    pub fn touch(&mut self, name: &str) -> Result<(), TrustManagerError> {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.last_used = Utc::now();

            // Persist updated metadata
            let meta_path = self.cache_dir.join(format!("{}.json", name));
            if meta_path.exists() {
                let content = fs::read_to_string(&meta_path)?;
                let existing: CacheMetadata = serde_json::from_str(&content)?;
                let updated = CacheMetadata {
                    last_used: entry.last_used,
                    ..existing
                };
                fs::write(&meta_path, serde_json::to_string_pretty(&updated)?)?;
            }
        }
        Ok(())
    }

    /// List all cached skills.
    pub fn list(&self) -> Vec<&SkillCacheEntry> {
        self.entries.values().collect()
    }

    /// Remove a skill from the cache.
    pub fn remove(&mut self, name: &str) -> Result<bool, TrustManagerError> {
        let _entry = match self.entries.remove(name) {
            Some(e) => e,
            None => return Ok(false),
        };

        // Remove metadata file
        let meta_path = self.cache_dir.join(format!("{}.json", name));
        if meta_path.exists() {
            fs::remove_file(&meta_path)?;
        }

        info!(skill = %name, "removed from cache");
        Ok(true)
    }

    /// Clean up stale cache entries (>30 days since last used).
    pub fn clean_stale(&mut self) -> Result<Vec<String>, TrustManagerError> {
        let threshold = Utc::now() - chrono::Duration::days(30);
        let mut removed = Vec::new();

        let names: Vec<String> = self.entries.keys().cloned().collect();
        for name in names {
            if let Some(entry) = self.entries.get(&name) {
                if entry.last_used < threshold {
                    self.remove(&name)?;
                    removed.push(name);
                }
            }
        }

        if !removed.is_empty() {
            info!(count = removed.len(), "cleaned stale cache entries");
        }

        Ok(removed)
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) -> Result<usize, TrustManagerError> {
        let count = self.entries.len();
        let names: Vec<String> = self.entries.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        info!(count, "cleared skill cache");
        Ok(count)
    }

    /// Load cached skill metadata from disk.
    fn load_metadata(
        cache_dir: &Path,
        entries: &mut HashMap<String, SkillCacheEntry>,
    ) -> Result<(), TrustManagerError> {
        if !cache_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(cache_dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(meta) = serde_json::from_str::<CacheMetadata>(&content) {
                    let tier = TrustTier::from_str(&meta.trust_tier).unwrap_or(TrustTier::Community);
                    let entry = SkillCacheEntry {
                        name: meta.name.clone(),
                        version: meta.version,
                        source: meta.source,
                        install_path: meta.install_path,
                        download_url: meta.download_url,
                        checksum: meta.checksum,
                        trust_tier: tier,
                        installed_at: meta.installed_at,
                        last_used: meta.last_used,
                    };
                    entries.insert(meta.name, entry);
                } else {
                    warn!(path = %path.display(), "failed to parse cache metadata");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_skill() {
        let dir = tempdir().unwrap();
        unsafe { std::env::set_var("HOME", dir.path()); }

        let mut cache = SkillCache::new().unwrap();
        let entry = cache
            .cache_skill(
                "test-skill",
                "1.0.0",
                "clawhub",
                dir.path().join("test-skill"),
                Some("https://example.com/skill.tar.gz".to_string()),
                Some("abc123".to_string()),
                TrustTier::Community,
            )
            .unwrap();

        assert_eq!(entry.name, "test-skill");
        assert_eq!(entry.version, "1.0.0");
        assert!(cache.is_cached("test-skill"));
    }

    #[test]
    fn test_remove_skill() {
        let dir = tempdir().unwrap();
        unsafe { std::env::set_var("HOME", dir.path()); }

        let mut cache = SkillCache::new().unwrap();
        cache
            .cache_skill(
                "test-skill",
                "1.0.0",
                "clawhub",
                dir.path().join("test-skill"),
                None,
                None,
                TrustTier::Community,
            )
            .unwrap();

        assert!(cache.remove("test-skill").unwrap());
        assert!(!cache.is_cached("test-skill"));
    }
}
