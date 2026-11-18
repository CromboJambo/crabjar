//! Tool execution framework for the orchestrator.
//! Each tool is implemented as a separate module with a consistent interface.

use agent_context::Store;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod browser_exec;
pub mod browser_login;
pub mod browser_vault;
pub mod by_source;
pub mod clarify;
pub mod delegate_task;
pub mod homeassistant;
pub mod manage_catalog;
pub mod memory;
pub mod patch;
pub mod process_manage;
pub mod react_to_message;
pub mod read_file;
pub mod recent_events;
pub mod run_command;
pub mod search_files;
pub mod search_logs;
pub mod session_search;
pub mod skills;
pub mod terminal;
pub mod video_analyze;
pub mod web_search;
pub mod write_file;

/// Context passed to tool handlers.
pub struct ToolContext {
    pub store: Arc<std::sync::Mutex<Store>>,
    pub process_registry: Arc<Mutex<crate::process_management::ProcessRegistry>>,
}

/// Execute a tool by name with the given arguments.
pub async fn execute(
    function_name: &str,
    args: &[String],
    ctx: &ToolContext,
) -> Result<String, String> {
    match function_name {
        "run_command" => run_command::handle(args).await,
        "search_logs" => search_logs::handle(args, ctx).await,
        "recent_events" => recent_events::handle(args, ctx).await,
        "by_source" => by_source::handle(args, ctx).await,
        "web_search" => web_search::handle(args).await,
        "memory" => memory::handle(args, ctx).await,
        "delegate_task" => delegate_task::handle(args, ctx).await,
        "clarify" => clarify::handle(args, ctx).await,
        "session_search" => session_search::handle(args, ctx).await,
        "process_manage" => process_manage::handle(args, ctx).await,
        "browser_exec" => browser_exec::handle(args, ctx).await,
        "browser_login" => browser_login::handle(args).await,
        "browser_vault" => browser_vault::handle(args).await,
        "manage_catalog" => manage_catalog::handle(args).await,
        "react_to_message" => react_to_message::handle(args).await,
        "video_analyze" => video_analyze::handle(args).await,
        "read_file" => read_file::handle(args).await,
        "write_file" => write_file::handle(args).await,
        "patch" => patch::handle(args).await,
        "search_files" => search_files::handle(args).await,
        "terminal" => terminal::handle(args).await,
        "skills_list" => skills::handle_skills_list(args).await,
        "skill_view" => skills::handle_skill_view(args).await,
        "skill_manage" => skills::handle_skill_manage(args).await,
        name if name.starts_with("ha_") => homeassistant::handle(name, args, ctx).await,
        _ => Err(format!("Unknown tool: {}", function_name)),
    }
}
