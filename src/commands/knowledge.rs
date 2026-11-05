use crate::knowledge_store::KnowledgeBridge;
use crate::knowledge_store::commands::KnowledgeCommandExt;
use crabjar_lib::KnowledgeCommand;

/// Handle knowledge commands
pub async fn handle_knowledge_command(
    command: KnowledgeCommand,
) -> Result<serde_json::Value, agent_context::Error> {
    let project_root = std::env::current_dir()
        .map_err(|err| agent_context::Error::Io(std::io::Error::other(err.to_string())))?;

    // Build guard DB path for gate
    let guard_db_path = project_root.join("guard/guard.db");
    let guard_db = crabjar_guard::GuardDb::open(&guard_db_path).unwrap_or_else(|_| {
        let temp_dir = tempfile::tempdir().unwrap();
        crabjar_guard::GuardDb::open(temp_dir.path().join("guard.db")).unwrap()
    });

    let bridge = KnowledgeBridge::new(
        project_root
            .join("memory/knowledge.db")
            .to_string_lossy()
            .as_ref(),
        project_root,
        None,
    )?
    .with_guard_db(guard_db);
    command.execute(&bridge).await
}
