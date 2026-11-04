/// Configuration for the CrabJar host runtime.
///
/// Loaded from ~/.config/crabjar-host/config.toml or a project-scoped path.
use serde::{Deserialize, Serialize};

/// Host configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostConfig {
    /// Host name / identifier
    pub name: String,
    /// Whether to run in debug mode (verbose logging, no optimization)
    pub debug: bool,
    /// Default plugin to load on startup
    pub default_plugin: Option<String>,
    /// WebView engine preference: "webview2" (Windows), "webkit" (Linux/macOS)
    pub webview_engine: String,
    /// Tray icon configuration
    pub tray: TrayConfig,
    /// Notification settings
    pub notifications: NotificationConfig,
    /// Agent loop settings
    pub agent: AgentConfig,
    /// SQLite database path
    pub db_path: String,
    /// Data directory for plugins
    pub data_dir: String,
}

/// System tray configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrayConfig {
    /// Show tray icon
    pub enabled: bool,
    /// Tray icon file path (relative to data dir)
    pub icon_path: Option<String>,
    /// Default tray menu items
    pub menu_items: Vec<String>,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            icon_path: None,
            menu_items: vec!["Show".into(), "Hide".into(), "Quit".into()],
        }
    }
}

/// Notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationConfig {
    /// Enable desktop notifications
    pub enabled: bool,
    /// Default timeout in seconds
    pub default_timeout: u32,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_timeout: 5,
        }
    }
}

/// Agent loop configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    /// Maximum loop iterations before pausing
    pub max_iterations: u32,
    /// Confidence threshold to auto-complete
    pub confidence_threshold: f32,
    /// Whether to persist WorkItems to SQLite
    pub persist_work_items: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            confidence_threshold: 0.85,
            persist_work_items: true,
        }
    }
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            name: "crabjar-host".into(),
            debug: false,
            default_plugin: None,
            webview_engine: "webkit".into(),
            tray: TrayConfig::default(),
            notifications: NotificationConfig::default(),
            agent: AgentConfig::default(),
            db_path: "crabjar-host.db".into(),
            data_dir: "data".into(),
        }
    }
}

impl HostConfig {
    /// Load config from a TOML file.
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed(path.into(), e))?;
        let config =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseFailed(path.into(), e))?;
        Ok(config)
    }

    /// Save config to a TOML file.
    pub fn to_file(&self, path: &str) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self).map_err(ConfigError::SerializeFailed)?;
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::WriteFailed(path.into(), e))?;
        }
        std::fs::write(path, contents).map_err(|e| ConfigError::WriteFailed(path.into(), e))?;
        Ok(())
    }

    /// Load config from file, or create default at the given path.
    ///
    /// Only creates a default config when the file is genuinely missing.
    /// Other read errors (permission denied, disk I/O failures) propagate
    /// so callers can distinguish "no config yet" from "config unreadable".
    pub fn load_or_default(path: &str) -> Result<Self, ConfigError> {
        match Self::from_file(path) {
            Ok(config) => Ok(config),
            Err(ref e @ ConfigError::ReadFailed(_, ref io_err)) if is_not_found(io_err) => {
                let default = Self::default();
                default.to_file(path)?;
                Ok(default)
            }
            Err(e) => Err(e),
        }
    }
}

/// Configuration errors.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config from '{0}': {1}")]
    ReadFailed(String, std::io::Error),
    #[error("failed to parse config from '{0}': {1}")]
    ParseFailed(String, toml::de::Error),
    #[error("failed to serialize config: {0}")]
    SerializeFailed(toml::ser::Error),
    #[error("failed to write config to '{0}': {1}")]
    WriteFailed(String, std::io::Error),
}

/// Check if an io error indicates the file does not exist.
fn is_not_found(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HostConfig::default();
        assert_eq!(config.name, "crabjar-host");
        assert!(!config.debug);
        assert_eq!(config.webview_engine, "webkit");
        assert!(config.tray.enabled);
        assert!(config.notifications.enabled);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = HostConfig::default();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        config.to_file(path).unwrap();
        let loaded = HostConfig::from_file(path).unwrap();

        assert_eq!(config.name, loaded.name);
        assert_eq!(config.debug, loaded.debug);
        assert_eq!(config.webview_engine, loaded.webview_engine);
        assert_eq!(config.tray.enabled, loaded.tray.enabled);
    }

    #[test]
    fn test_load_or_default_creates_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.toml");

        let config = HostConfig::load_or_default(path.to_str().unwrap()).unwrap();
        assert_eq!(config.name, "crabjar-host");

        // File should now exist with default content
        assert!(path.exists());
    }

    #[test]
    fn test_load_or_default_propagates_read_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readonly.toml");

        // Create file with some content, then make it unreadable
        std::fs::write(&path, "existing content").unwrap();

        // On Unix, chmod 000 makes it unreadable; on Windows this is harder.
        // Use a simpler approach: create in a directory that's not writable.
        let readonly_dir = dir.path().join("readonly");
        std::fs::create_dir(&readonly_dir).unwrap();
        let ro_path = readonly_dir.join("config.toml");
        std::fs::write(&ro_path, "name = \"test\"").unwrap();

        // Try to read from a path in a directory we can't write to for defaults
        // Actually, test the real scenario: file exists but has read error
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&ro_path).unwrap().permissions();
            let new_perms = perms.mode() & 0o777 & !0o444; // remove read bits
            std::fs::set_permissions(&ro_path, std::fs::Permissions::from_mode(new_perms)).unwrap();

            let result = HostConfig::load_or_default(ro_path.to_str().unwrap());
            assert!(result.is_err(), "should fail on permission denied");

            // Restore permissions to allow cleanup
            std::fs::set_permissions(&ro_path, perms).unwrap();
        }
    }
}
