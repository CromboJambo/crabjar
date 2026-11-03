use std::path::Path;

/// Detected init system types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitSystem {
    Dinit,
    Runit,
    Systemd,
    None,
}

impl std::fmt::Display for InitSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitSystem::Dinit => write!(f, "dinit"),
            InitSystem::Runit => write!(f, "runit"),
            InitSystem::Systemd => write!(f, "systemd"),
            InitSystem::None => write!(f, "none"),
        }
    }
}

/// Detect the init system running on this machine.
/// Returns a priority-ordered list of detected init systems.
#[must_use]
pub fn detect_init_systems() -> Vec<InitSystem> {
    let mut detected = Vec::new();

    // Check for dinit
    if Path::new("/etc/dinit.d").exists() || which("dinitctl").is_some() {
        detected.push(InitSystem::Dinit);
    }

    // Check for runit
    if Path::new("/etc/sv").exists() || Path::new("/var/service").exists() || which("sv").is_some() {
        detected.push(InitSystem::Runit);
    }

    // Check for systemd
    if Path::new("/run/systemd/system").exists() || which("systemctl").is_some() {
        detected.push(InitSystem::Systemd);
    }

    if detected.is_empty() {
        detected.push(InitSystem::None);
    }

    detected
}

/// Get the primary init system (first in priority order).
#[must_use]
pub fn detect_primary_init() -> InitSystem {
    let systems = detect_init_systems();
    systems[0].clone()
}

/// Check if a specific binary is available on PATH.
fn which(binary: &str) -> Option<String> {
    let result = std::process::Command::new("which")
        .arg(binary)
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_non_empty() {
        let systems = detect_init_systems();
        assert!(!systems.is_empty(), "Should always detect at least 'none'");
    }

    #[test]
    fn test_primary_init_is_valid() {
        let primary = detect_primary_init();
        // Just verify it doesn't panic
        let _ = format!("{}", primary);
    }
}