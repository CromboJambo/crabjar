//! Training data preprocessing: strip secret-shaped objects from session exports.
//!
//! Detects SSH keys, API tokens, email addresses, IP addresses, and file paths
//! containing usernames, replacing them with `<SECRET:type>` placeholders.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error types for secret stripping operations.
#[derive(Debug, Error)]
pub enum StripError {
    #[error("regex compilation failed: {0}")]
    Regex(#[from] regex::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    Config(String),
}

/// A secret pattern to detect and strip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    /// Identifier used in the replacement placeholder (`<SECRET:type>`).
    pub r#type: String,
    /// Regex pattern to match secrets of this type.
    pub pattern: String,
    /// Whether this pattern should be case-insensitive.
    #[serde(default)]
    pub case_insensitive: bool,
}

/// Configuration for the secret stripper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripConfig {
    /// Patterns to detect and strip.
    pub patterns: Vec<SecretPattern>,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            patterns: vec![
                SecretPattern {
                    r#type: "ssh_key".to_string(),
                    pattern: "ssh-(ed25519|rsa|dss)\\s+[A-Za-z0-9+/=]{20,}".to_string(),
                    case_insensitive: false,
                },
                SecretPattern {
                    r#type: "api_token".to_string(),
                    pattern: "[a-zA-Z0-9_]{32,}".to_string(),
                    case_insensitive: false,
                },
                SecretPattern {
                    r#type: "email".to_string(),
                    pattern: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}".to_string(),
                    case_insensitive: true,
                },
                SecretPattern {
                    r#type: "ipv4".to_string(),
                    // Simplified: no look-around (not supported by regex crate)
                    pattern: "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}".to_string(),
                    case_insensitive: false,
                },
                SecretPattern {
                    r#type: "home_path".to_string(),
                    pattern: "/home/[a-zA-Z][a-zA-Z0-9_-]+/?".to_string(),
                    case_insensitive: false,
                },
            ],
        }
    }
}

/// Result of stripping secrets from text.
#[derive(Debug)]
pub struct StripResult {
    /// The cleaned text with secrets replaced by placeholders.
    pub cleaned: String,
    /// Map of secret type → count of occurrences found.
    pub secrets_found: HashMap<String, usize>,
}

/// Compile patterns into regexes for efficient repeated use.
struct CompiledPatterns {
    compiled: Vec<(String, Regex)>,
}

impl CompiledPatterns {
    fn new(config: &StripConfig) -> Result<Self, StripError> {
        let mut compiled = Vec::new();
        for pattern in &config.patterns {
            let flags = if pattern.case_insensitive { "(?i)" } else { "" };
            let full_pattern = format!("{}{}", flags, pattern.pattern);
            let regex = Regex::new(&full_pattern)?;
            compiled.push((pattern.r#type.clone(), regex));
        }
        Ok(Self { compiled })
    }

    fn strip(&self, text: &str) -> StripResult {
        let mut result = text.to_string();
        let mut secrets_found = HashMap::new();

        for (r#type, regex) in &self.compiled {
            let count = regex.find_iter(&result).count();
            if count > 0 {
                secrets_found.insert(r#type.clone(), count);
                result = regex
                    .replace_all(&result, &format!("<SECRET:{}>", r#type))
                    .to_string();
            }
        }

        StripResult {
            cleaned: result,
            secrets_found,
        }
    }
}

/// Strip secrets from text using the default configuration.
pub fn strip_secrets(text: &str) -> Result<StripResult, StripError> {
    strip_with_config(text, &StripConfig::default())
}

/// Strip secrets from text using a custom configuration.
pub fn strip_with_config(text: &str, config: &StripConfig) -> Result<StripResult, StripError> {
    let compiled = CompiledPatterns::new(config)?;
    Ok(compiled.strip(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ssh_keys() {
        let text = "My key is ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGv something";
        let result = strip_secrets(text).unwrap();
        assert!(result.cleaned.contains("<SECRET:ssh_key>"));
        assert_eq!(result.secrets_found.get("ssh_key"), Some(&1));
    }

    #[test]
    fn strips_email_addresses() {
        let text = "Contact me at user@example.com for details";
        let result = strip_secrets(text).unwrap();
        assert!(result.cleaned.contains("<SECRET:email>"));
        assert!(!result.cleaned.contains("user@example.com"));
    }

    #[test]
    fn strips_ipv4_addresses() {
        let text = "Server at 192.168.1.100 is down";
        let result = strip_secrets(text).unwrap();
        assert!(result.cleaned.contains("<SECRET:ipv4>"));
        assert!(!result.cleaned.contains("192.168.1.100"));
    }

    #[test]
    fn strips_home_paths() {
        let text = "File is at /home/crombo/projects/active/crabjar";
        let result = strip_secrets(text).unwrap();
        assert!(result.cleaned.contains("<SECRET:home_path>"));
        assert!(!result.cleaned.contains("/home/crombo"));
    }

    #[test]
    fn strips_api_tokens() {
        let text = "Token: abcdefghijklmnopqrstuvwxyz1234567890";
        let result = strip_secrets(text).unwrap();
        assert!(result.cleaned.contains("<SECRET:api_token>"));
    }

    #[test]
    fn counts_multiple_occurrences() {
        let text = "Emails: a@b.com and c@d.com";
        let result = strip_secrets(text).unwrap();
        assert_eq!(result.secrets_found.get("email"), Some(&2));
    }

    #[test]
    fn no_secrets_leaves_text_unchanged() {
        let text = "This is clean text with no secrets.";
        let result = strip_secrets(text).unwrap();
        assert_eq!(result.cleaned, text);
        assert!(result.secrets_found.is_empty());
    }

    #[test]
    fn custom_config_works() {
        let config = StripConfig {
            patterns: vec![SecretPattern {
                r#type: "custom".to_string(),
                pattern: "CUSTOM-[A-Z]{5}".to_string(),
                case_insensitive: false,
            }],
        };
        let text = "Code CUSTOM-ABCDE here";
        let result = strip_with_config(text, &config).unwrap();
        assert!(result.cleaned.contains("<SECRET:custom>"));
    }
}
