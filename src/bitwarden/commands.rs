/// Bitwarden subcommand handlers.
///
/// Extracted from main.rs to reduce bloat.

use serde_json::json;

use crate::bitwarden;
use crate::BitwardenCommand;

/// Handle bitwarden commands
pub fn handle_bitwarden_command(
    command: BitwardenCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        BitwardenCommand::Status => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "status": "not_available",
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let status = bitwarden::cli::status()?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "status": "available",
                    "session": status,
                },
            }))
        }
        BitwardenCommand::List { folder, collection } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "items": [],
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let items = bitwarden::cli::list_items(folder.as_deref(), collection.as_deref())?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "items": items,
                },
            }))
        }
        BitwardenCommand::Get { id } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "item": null,
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let item = bitwarden::cli::get_item(&id)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "item": item,
                },
            }))
        }
        BitwardenCommand::Search { query } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "items": [],
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let items = bitwarden::cli::search_items(&query)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "items": items,
                    "query": query,
                },
            }))
        }
        BitwardenCommand::Generate {
            length,
            uppercase,
            lowercase,
            numbers,
            special,
        } => {
            if !bitwarden::cli::is_available() {
                return Ok(json!({
                    "success": false,
                    "bitwarden": {
                        "password": null,
                        "error": "bitwarden CLI not found",
                    },
                }));
            }

            let password =
                bitwarden::cli::generate_password(length, uppercase, lowercase, numbers, special)?;
            Ok(json!({
                "success": true,
                "bitwarden": {
                    "password": password,
                },
            }))
        }
    }
}
