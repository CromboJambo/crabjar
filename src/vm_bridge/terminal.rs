/// Terminal integration for vm-bridge
///
/// Provides shared terminal support via:
/// - wezterm (SSH, CLI)
/// - zellij (terminal multiplexer)

use anyhow::Result;

/// Terminal multiplexer for shared terminal sessions
pub enum TerminalMultiplexer {
    /// wezterm (SSH, CLI)
    Wezterm,
    /// zellij (terminal multiplexer)
    Zellij,
}

impl TerminalMultiplexer {
    /// Create a new shared terminal session
    pub async fn new_session(&self, name: &str) -> Result<()> {
        match self {
            Self::Wezterm => {
                // Create wezterm session
                let output = tokio::process::Command::new("wezterm")
                    .args(&["start", "--", "bash", "-c", f"tmux new-session -d -s {name} && tmux send-keys -t {name} 'cd "${CRABJAR_ROOT:-.}" && cargo run' Enter"])
                    .output()
                    .await?;
                
                if !output.status.success() {
                    anyhow::bail!("Failed to create wezterm session: {}", String::from_utf8_loss_of(&output.stderr));
                }
            }
            Self::Zellij => {
                // Create zellij session
                let output = tokio::process::Command::new("zellij")
                    .args(&["new-session", "-s", name, "--", "bash", "-c", f"cd "${CRABJAR_ROOT:-.}" && cargo run"])
                    .output()
                    .await?;
                
                if !output.status.success() {
                    anyhow::bail!("Failed to create zellij session: {}", String::from_utf8_loss_of(&output.stderr));
                }
            }
        }
        
        Ok(())
    }
}
