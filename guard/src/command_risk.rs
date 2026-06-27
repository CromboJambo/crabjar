//! Risk assessment for commands — high/medium/low risk classification.
//!
//! Contains command risk lists and the `CommandRisk` enum.

//! High-risk commands that are always blocked.
pub const HIGH_RISK_COMMANDS: &[&str] = &[
    "rm",
    "remove",
    "del",
    "delete",
    "unlink",
    "sudo",
    "su",
    "chmod",
    "chown",
    "mkfs",
    "fdisk",
    "dd",
    "iptables",
    "kill",
    "killall",
    "shutdown",
    "reboot",
    "halt",
    "format",
    "curl",
    "wget",
    "nc",
    "netcat",
    "socat",
    "cp",
    "mv",
    "tar",
    "zip",
    "unzip",
    "pip install",
    "npm install",
    "cargo install",
    "apt",
    "apt-get",
    "yum",
    "dnf",
    "pacman",
];

/// Medium-risk commands that require review.
pub const MEDIUM_RISK_COMMANDS: &[&str] = &[
    "git",
    "clone",
    "checkout",
    "branch",
    "docker",
    "podman",
    "ssh",
    "scp",
    "rsync",
    "vim",
    "vi",
    "nano",
    "emacs",
    "cargo",
    "rustc",
    "python",
    "pip",
    "node",
    "npm",
    "npx",
];

/// Risk level for a command. Higher risk means more scrutiny.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    Low,
    Medium,
    High,
    Unauthorized,
}

impl CommandRisk {
    /// Returns `true` if this risk level requires human review.
    pub fn requires_review(&self) -> bool {
        matches!(self, CommandRisk::Medium | CommandRisk::High)
    }

    /// Returns `true` if this risk level blocks execution.
    pub fn blocks_execution(&self) -> bool {
        matches!(self, CommandRisk::High | CommandRisk::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_risk_commands_blocked() {
        assert!(CommandRisk::High.blocks_execution());
        assert!(!CommandRisk::High.requires_review());
    }

    #[test]
    fn medium_risk_requires_review() {
        assert!(CommandRisk::Medium.requires_review());
        assert!(!CommandRisk::Medium.blocks_execution());
    }

    #[test]
    fn low_risk_neither() {
        assert!(!CommandRisk::Low.requires_review());
        assert!(!CommandRisk::Low.blocks_execution());
    }

    #[test]
    fn unauthorized_blocks_execution() {
        assert!(CommandRisk::Unauthorized.blocks_execution());
        assert!(!CommandRisk::Unauthorized.requires_review());
    }

    #[test]
    fn high_risk_commands_list() {
        assert!(HIGH_RISK_COMMANDS.contains(&"rm"));
        assert!(HIGH_RISK_COMMANDS.contains(&"sudo"));
        assert!(HIGH_RISK_COMMANDS.contains(&"chmod"));
    }

    #[test]
    fn medium_risk_commands_list() {
        assert!(MEDIUM_RISK_COMMANDS.contains(&"git"));
        assert!(MEDIUM_RISK_COMMANDS.contains(&"docker"));
        assert!(MEDIUM_RISK_COMMANDS.contains(&"ssh"));
    }
}
