//! Tool registry CLI subcommand handlers.
//!
//! Provides `crabjar tool list` and `crabjar tool discover` commands.

use crabjar_lib::ToolCommand;

/// Handle tool registry commands
pub async fn handle_tool_command(
    command: ToolCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        ToolCommand::List { r#type, limit } => {
            crate::commands::tool::list::handle_tool_list(r#type, limit)
        }
        ToolCommand::Discover { source } => {
            crate::commands::tool::discover::handle_tool_discover(source).await
        }
        ToolCommand::SkillInstall { path } => {
            crate::commands::tool::skill::handle_skill_install(path)
        }
        ToolCommand::PluginEnable { name } => {
            crate::commands::tool::skill::handle_plugin_enable(name)
        }
        ToolCommand::PluginDisable { name } => {
            crate::commands::tool::skill::handle_plugin_disable(name)
        }
        ToolCommand::ToolUninstall { name } => {
            crate::commands::tool::skill::handle_tool_uninstall(name)
        }
        ToolCommand::Search {
            query,
            stars,
            sort,
            limit,
        } => crate::commands::tool::github::handle_github_search(query, stars, sort, limit).await,
        ToolCommand::Install { repo, target } => {
            crate::commands::tool::github::handle_github_install(repo, target).await
        }
        ToolCommand::ClawhubSearch { query, limit } => {
            crate::commands::tool::clawhub::handle_clawhub_search(query, limit).await
        }
        ToolCommand::ClawhubInstall {
            name,
            version,
            target,
        } => crate::commands::tool::clawhub::handle_clawhub_install(name, version, target).await,
        ToolCommand::Scan { path, checksum } => {
            crate::commands::tool::clawhub::handle_skill_scan(path, checksum)
        }
        ToolCommand::Cache { action } => crate::commands::tool::cache::handle_cache_command(action),
        ToolCommand::Updates { name } => crate::commands::tool::cache::handle_updates(name),
        ToolCommand::Auth => crate::commands::tool::cache::handle_auth(),
    }
}
