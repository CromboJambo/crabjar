use crate::command_risk::CommandRisk;
use crate::risk_config::RiskConfig;

/// Assess command risk based on name AND arguments (dynamic, context-aware).
/// This addresses redteam finding #5: risk must be evaluated based on what the
/// command is operating on, not just the command name.
pub fn assess_command_risk(
    command: &str,
    args: &[String],
    risk_config: &RiskConfig,
) -> CommandRisk {
    let basename = command.split('/').next_back().unwrap_or(command);

    // Dynamic risk assessment: evaluate both command name and arguments
    // to determine actual risk level based on context.

    // Check for high-risk patterns in arguments regardless of command name
    let args_str = args.join(" ");

    // rm -rf / or similar destructive patterns are always high risk
    if basename.eq_ignore_ascii_case("rm") {
        if args_str.contains("-rf /") || args_str.contains("-fr /") || args_str == "/" {
            return CommandRisk::High;
        }
        // rm with specific files is lower risk
        if !args.is_empty() && !args_str.contains("/") {
            return CommandRisk::Low;
        }
    }

    // chmod/chown on critical paths is high risk
    if (basename.eq_ignore_ascii_case("chmod") || basename.eq_ignore_ascii_case("chown"))
        && (args_str.contains("/etc/") || args_str.contains("/usr/") || args_str.contains("/"))
    {
        return CommandRisk::High;
    }

    // Package manager install commands are high risk (system-wide changes)
    if [
        "apt", "apt-get", "yum", "dnf", "pacman", "pip", "npm", "cargo",
    ]
    .iter()
    .any(|c| basename.eq_ignore_ascii_case(c))
        && (args_str.contains("install") || args_str.contains("upgrade"))
    {
        return CommandRisk::High;
    }

    // Check static high-risk list (for commands that are always risky)
    for risk_cmd in &risk_config.high_risk {
        if basename.eq_ignore_ascii_case(risk_cmd) {
            return CommandRisk::High;
        }
    }

    // Check static medium-risk list
    for risk_cmd in &risk_config.medium_risk {
        if basename.eq_ignore_ascii_case(risk_cmd) {
            return CommandRisk::Medium;
        }
    }

    CommandRisk::Low
}
