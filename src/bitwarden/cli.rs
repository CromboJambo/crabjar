// crabjar/src/bitwarden/cli.rs
// Bitwarden CLI interaction functions

use crate::bitwarden::{BitwardenError, BitwardenItem, BitwardenResult};
use serde_json;
use std::process::Command;

/// Check if bitwarden CLI is available
pub fn is_available() -> bool {
    Command::new("bw")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Login to bitwarden CLI
pub fn login(email: &str, master_password: &str, server: Option<&str>) -> BitwardenResult<()> {
    let mut cmd = Command::new("bw");
    cmd.arg("login").arg(email).arg(master_password);

    if let Some(server) = server {
        cmd.arg("--server").arg(server);
    }

    let output = cmd.output().map_err(|e| {
        BitwardenError::CliNotFound(format!("Failed to execute bw: {}", e))
    })?;
    
    if output.status.success() {
        Ok(())
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Logout from bitwarden CLI
pub fn logout() -> BitwardenResult<()> {
    let output = Command::new("bw").arg("logout").output().map_err(|e| {
        BitwardenError::CliError(format!("Failed to execute bw logout: {}", e))
    })?;
    
    if output.status.success() {
        Ok(())
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Get status of bitwarden session
pub fn status() -> BitwardenResult<String> {
    let output = Command::new("bw").arg("status").output().map_err(|e| {
        BitwardenError::CliError(format!("Failed to execute bw status: {}", e))
    })?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// List bitwarden items
pub fn list_items(folder: Option<&str>, collection: Option<&str>) -> BitwardenResult<Vec<BitwardenItem>> {
    let mut cmd = Command::new("bw");
    cmd.arg("list")
        .arg("items")
        .arg("--output")
        .arg("json");

    if let Some(folder) = folder {
        cmd.arg("--folderid").arg(folder);
    }

    if let Some(collection) = collection {
        cmd.arg("--collectionid").arg(collection);
    }

    let output = cmd.output().map_err(|e| {
        BitwardenError::CliError(format!("Failed to execute bw list: {}", e))
    })?;
    
    if output.status.success() {
        let items: Vec<BitwardenItem> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
        Ok(items)
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Get a specific bitwarden item by ID
pub fn get_item(id: &str) -> BitwardenResult<BitwardenItem> {
    let output = Command::new("bw")
        .arg("get")
        .arg("item")
        .arg(id)
        .arg("--output")
        .arg("json")
        .output()
        .map_err(|e| {
            BitwardenError::CliError(format!("Failed to execute bw get: {}", e))
        })?;

    if output.status.success() {
        let item: BitwardenItem = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
        Ok(item)
    } else {
        Err(BitwardenError::NotFound(format!(
            "Item {} not found",
            id
        )))
    }
}

/// Search bitwarden items by name
pub fn search_items(query: &str) -> BitwardenResult<Vec<BitwardenItem>> {
    let output = Command::new("bw")
        .arg("search")
        .arg(query)
        .arg("--output")
        .arg("json")
        .output()
        .map_err(|e| {
            BitwardenError::CliError(format!("Failed to execute bw search: {}", e))
        })?;

    if output.status.success() {
        let items: Vec<BitwardenItem> = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
        Ok(items)
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Generate a password via bitwarden CLI
pub fn generate_password(
    length: u32,
    uppercase: bool,
    lowercase: bool,
    numbers: bool,
    special: bool,
) -> BitwardenResult<String> {
    let mut cmd = Command::new("bw");
    cmd.arg("generate")
        .arg("--length")
        .arg(length.to_string());

    if uppercase {
        cmd.arg("--uppercase");
    }
    if lowercase {
        cmd.arg("--lowercase");
    }
    if numbers {
        cmd.arg("--numbers");
    }
    if special {
        cmd.arg("--special");
    }

    let output = cmd.output().map_err(|e| {
        BitwardenError::CliError(format!("Failed to execute bw generate: {}", e))
    })?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(BitwardenError::CliError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}
