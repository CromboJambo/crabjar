//! Machine management tools: list, status, add.

use crate::types::{AddMachineRequest, ListMachinesRequest, MachineStatusRequest};
use rmcp::handler::server::wrapper::Parameters;

pub fn list_machines(params: Parameters<ListMachinesRequest>) -> String {
    let req = params.0;

    if req.verbose {
        match crate::run_cmd_verbose("herdr machine list") {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd("herdr machine list") {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

pub fn machine_status(params: Parameters<MachineStatusRequest>) -> String {
    let req = params.0;

    let cmd = match &req.machine {
        Some(machine) => format!("herdr machine status {}", machine),
        None => "herdr machine status".to_string(),
    };

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

pub fn add_machine(params: Parameters<AddMachineRequest>) -> String {
    let req = params.0;

    let cmd = format!(
        "herdr machine add {} --label {}",
        req.ssh_target, req.label
    );

    if req.verbose {
        match crate::run_cmd_verbose(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    } else {
        match crate::run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
