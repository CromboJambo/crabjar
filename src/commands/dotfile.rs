use crabjar_lib::{DotfileCommand, HabitatCommand};
use crate::dotfile_manager::DotfileManager;
use serde_json::json;

pub fn handle(
    command: DotfileCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let project_root = std::env::current_dir()?;
    let manager = DotfileManager::new(project_root);

    match command {
        DotfileCommand::Promote { path } => manager.propose(&path, &path),
    }
}