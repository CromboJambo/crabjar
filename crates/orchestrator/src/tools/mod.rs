//! Tool execution framework for the orchestrator.
//! Each tool is implemented as a separate module with a consistent interface.

use std::sync::Arc;
use tokio::sync::Mutex;
use agent_context::Store;

pub mod run_command;
pub mod search_logs;
pub mod recent_events;
pub mod by_source;
pub mod web_search;
pub mod memory;
pub mod delegate_task;
pub mod clarify;
pub mod session_search;
pub mod process_manage;
pub mod browser_exec;
pub mod homeassistant;

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
        name if name.starts_with("ha_") => homeassistant::handle(name, args, ctx).await,
        _ => Err(format!("Unknown tool: {}", function_name)),
    }
}
